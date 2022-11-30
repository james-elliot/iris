//use geocoding::{Openstreetmap, Forward, Point};
use geocoding::Point;
use geojson::{GeoJson};
use geo_types::Geometry;
use std::convert::TryInto;
use std::fs;
use geo::algorithm::contains::Contains;
use std::io::{BufWriter,BufReader};
use std::time::{Instant,SystemTime};
use serde::{Serialize,Deserialize};
use std::fs::File;
use csv::{ReaderBuilder,WriterBuilder};
use ngrammatic::{CorpusBuilder, Pad};
use rust_fuzzy_search::fuzzy_compare;
use regex::Regex;
use lazy_static::lazy_static;


#[derive(Debug, Serialize, Deserialize, Clone)]
struct Adresse {
//    id:String,
//    id_fantoir:String,
    numero:Option<i32>,
    rep:String,
    nom_voie:String,
    code_postal:i32,
//    code_insee:String,
    nom_commune:String,
//    code_insee_ancienne_commune:String,
//    nom_ancienne_commune:String,
//    x:String,
//    y:String,
    lon:f64,
    lat:f64,
//    type_position:String,
//    alias:String,
//    nom_ld:String,
//    libelle_acheminement:String,
//    nom_afnor:String,
//    source_position:String,
//    source_nom_voie:String,
//    certification_commune:String,
//    cad_parcelles:String
}

fn read_adresses(file_path: &str) -> Vec<Adresse> {
    let now = Instant::now();
    let mut tab:Vec<Adresse> = Vec::new();
    let file = File::open(file_path).unwrap();
    let mut rdr = ReaderBuilder::new().delimiter(b';').from_reader(file);
    let mut nbi = 0;
    let mut nb = 0;
    for result in rdr.deserialize() {
	nb+=1;
	match result {
	    Ok(r) => tab.push(r),
	    Err (_) =>nbi+=1
	}
    }
    println!("Addresses read and parsed in {:?}, {:} records, {:} invalid records", now.elapsed(),nb,nbi);
    let now = Instant::now();
    tab.sort_unstable_by(|a, b| a.code_postal.cmp(&b.code_postal));
    println!("Addresses sorted in {:?}", now.elapsed());
    tab
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code,non_snake_case)]
struct Patient {
    N_PATIENT: String,
    PST_ADRESSE: String,
    PST_CP: String,
    PST_VILLE: String
}

#[derive(Debug, Clone, Serialize)]
struct Upatient {
    patient: String,
    adresse: String,
    cp: String,
    ville: String,
    n_adresse: String,
    n_cp: i32,
    n_ville: String,
    iris: String,
    s_ville: f32,
    s_adresse: f32
}

fn read_patients(file_path: &str) -> Vec<Patient> {
    let mut tab = Vec::new();
    let file = File::open(file_path).unwrap();
    let mut rdr = ReaderBuilder::new().delimiter(b';').from_reader(file);
    for result in rdr.deserialize() {tab.push(result.unwrap());}
    tab
}

fn write_patients(file_path: &str,v:Vec<Patient>) {
    let file = File::create(file_path).unwrap();
    let mut wrt = WriterBuilder::new().delimiter(b';').from_writer(file);
    for o in v {
	wrt.serialize(&o).unwrap();
    }
    wrt.flush().unwrap();
}

fn write_upatients(file_path: &str,v:Vec<Upatient>) {
    let file = File::create(file_path).unwrap();
    let mut wrt = WriterBuilder::new().delimiter(b';').from_writer(file);
    for o in v {
	wrt.serialize(&o).unwrap();
    }
    wrt.flush().unwrap();
}

#[derive(Debug,Clone)]
#[allow(dead_code)]
struct Maille {
    geom     : Geometry<f64>,
    pop      : Option<i64>, // Population
    dcomiris : Option<String>, // Code iris
    rev_med  : Option<f64>, // Revenu median
    tx_bac   : Option<f64>, // Taux de bachelier
    tx_chom  : Option<f64>, // Taux de chomage
    tx_ouvr  : Option<f64>  // Taux d'ouvriers
}

fn read_iris() -> Vec<Maille> {
    let now = Instant::now();
    let contents = fs::read_to_string("indice-de-defavorisation-sociale-fdep-par-iris.geojson")
	.expect("Something went wrong reading the Iris file");
    let geojson = contents.parse::<GeoJson>().unwrap();
    let mut tab = Vec::new();
    match geojson {
        GeoJson::FeatureCollection(ctn) => {
	    let f = ctn.features;
            println!("Found {} features", f.len());
	    for a in &f {
		let dcomiris = a.property("c_dcomiris").map(|v| v.as_str().unwrap().to_string());
		let tx_chom  = a.property("t1_txchom0").map(|v| v.as_f64().unwrap());
		let tx_ouvr  = a.property("t1_txouvr0").map(|v| v.as_f64().unwrap());
		let pop  = a.property("t1_p09_pop").map(|v| v.as_i64().unwrap());
		let tx_bac  = a.property("t1_txbac09").map(|v| v.as_f64().unwrap());
		let rev_med  = a.property("t1_rev_med").map(|v| v.as_f64().unwrap());
		let geom : Geometry<f64> = a.geometry.as_ref().unwrap().try_into().unwrap();
		tab.push(Maille {geom,dcomiris,pop,rev_med,tx_bac,tx_chom,tx_ouvr});
	    }
	    println!("Iris database built in {:?}", now.elapsed());
	}
	_ => panic!("No collection in Iris database")
    }
    tab
}

fn find_voies (v:&str) -> (String,String) {
    //    println!("Submitted:{:}",v);
    static VOIES:[(&str, &str); 14] = [
	(r"\brue\b","rue"),
	(r"\bavenue\b","avenue"),
	(r"\bboulevard\b","boulevard"),
	(r"\ballees\b","allees"),
	(r"\bblv\b","boulevard"),
	(r"\bblvd\b","boulevard"),
	(r"\ballee\b","allee"),
	(r"\bav\b","avenue"),
	(r"\bch\b","chemin"),
	(r"\bimp\b","impasse"),
	(r"\broute\b","route"),
	(r"\bimpasse\b","impasse"),
	(r"\bpassage\b","passage"),
	(r"\bchemin\b","chemin"),
    ];
    lazy_static! {
	static ref T:[Vec<Regex>;2]={
	    let mut t1 = Vec::new();
	    let mut t2 = Vec::new();
	    for a in &VOIES {
		let re = Regex::new(a.0).unwrap();
		t1.push(re);
		let re = Regex::new(&a.0[2..]).unwrap();
		t2.push(re);
	    }
	    [t1,t2]
	};
    }
    for i in 0..T.len() {
	for (j,re) in T[i].iter().enumerate() {
	    if let Some(m) = re.find(v) {
		let start = m.start();
		let end = m.end();
		if i==0 || (start > 0 && v.chars().nth(start-1).unwrap().is_ascii_digit()) {
		    let first = v[0..start].to_owned();
		    let mut last = VOIES[j].1.to_string();
		    last.push_str(&v[end..]);
		    return (first,last)
		}
	    }
	}
    }
    (v.to_owned(),"".to_owned())
}

fn find_num(v: &str) -> i32 {
    for i in (0..v.len()).rev() {
	let c = v.chars().nth(i).unwrap();
	if c.is_ascii_digit() {
	    for j in (0..i).rev() {
		let c = v.chars().nth(j).unwrap();
		if ! c.is_ascii_digit() {
		    let num=v[j+1..i+1].parse::<i32>().unwrap();
		    return num;
		}
	    }
	    let num=v[0..i+1].parse::<i32>().unwrap();
	    return num;
	}
    }
    0
}

fn remove_last(v:String)->String {
    let lasts=["app ","appt ","apt ","appartement ","bat ","batiment "];
    for a in &lasts {
	if let Some(i) = v.find(a) {
	    for j in (0..i).rev() {
		if v.chars().nth(j).unwrap()!= ' ' {
		    return v[0..j+1].to_owned();
		}
	    }
	    return v[0..i].to_owned();
	}
    }
    v
}

fn extract_info(r:&str)-> (i32,String) {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[0-9]").unwrap();
	static ref NRE:Regex = Regex::new(r"[^0123456789 ]").unwrap();
	static ref RE4:Regex = Regex::new(r"[ ]").unwrap();
    }
    let v = normalize_street(r);
    let (first,last)=find_voies(&v); 
    if ! last.is_empty() {
	let num = find_num(&first);
	let v=remove_last(last);
	return (num,v);
    }
    let res = RE.find(&v);
    let nres = NRE.find(&v);
    if let (Some (r1),Some(r2)) = (res,nres) {
	let i = r1.start();
	let j = r2.start();
	if j > i {
	    let k=RE4.find(&v[i..]).unwrap().start();
	    let num = v[i..k].parse::<i32>().unwrap();
	    let v = remove_last(v[j..].to_string());
	    return (num,v);
	}
    }
    (0,v)
}

fn find_first_last_cp(addrs:&[Adresse],cp:i32,f:i32,v:&mut Vec<usize>) -> bool {
    if cp==0 {return false;}
    if let Ok(p)=addrs.binary_search_by(|c| (c.code_postal/f).cmp(&(cp/f))) {
	for i in (0..p).rev() {
	    if addrs[i].code_postal/f != cp/f {break;}
	    v.push(i);
	}
	for (i,o) in addrs.iter().enumerate().skip(p) {
	    if o.code_postal/f != cp/f {break;}
	    v.push(i);
	}
	true
    }
    else {false}
}

#[allow(dead_code)]
fn find_vec_city(addrs:&[Adresse],cp:i32,city:String,v:&mut Vec<usize>) -> bool {
    for (i,o) in addrs.iter().enumerate() {
	if city.eq(&o.nom_commune) && (cp==0 || (cp/1000)==(o.code_postal/1000)) {v.push(i);}
    }
    ! v.is_empty()
}

fn get_addrs(street:&String,num:i32,cp:i32,city:&String,addrs:&[Adresse])->Option<usize> {
    let mut text = String::new();
    let mut ind = 0;
    let mut tab = Vec::new();
    let mut ntab = Vec::new();
    if find_first_last_cp(addrs,cp,1,&mut tab)  {
	ind = *tab.iter().max_by_key(
	    |x| (100.0*fuzzy_compare(&city,&addrs[**x].nom_commune)) as i64
	).unwrap();
	if fuzzy_compare(&city,&addrs[ind].nom_commune) > 0.8 {
	    let mut corpus = CorpusBuilder::new().arity(2).pad_full(Pad::Auto).finish();
	    for o in &tab {
		if addrs[ind].nom_commune.eq(&addrs[*o].nom_commune) {
		    corpus.add_text(&addrs[*o].nom_voie);
		    ntab.push(*o);
		}
	    }
	    if let Some(t)=corpus.search(&street, 0.8).first() {text.push_str(&t.text);}
	}
    }
    if text.is_empty() && find_first_last_cp(addrs,cp,1000,&mut tab)  {
	ind = *tab.iter().max_by_key(
	    |x| (100.0*fuzzy_compare(&city,&addrs[**x].nom_commune)) as i64
	).unwrap();
	if fuzzy_compare(&city,&addrs[ind].nom_commune) > 0.8 {
	    let mut corpus = CorpusBuilder::new().arity(2).pad_full(Pad::Auto).finish();
	    for o in &tab {
		if addrs[ind].nom_commune.eq(&addrs[*o].nom_commune) {
		    corpus.add_text(&addrs[*o].nom_voie);
		    ntab.push(*o);
		}
	    }
	    if let Some(t)=corpus.search(&street, 0.8).first() {text.push_str(&t.text);}
	}
    }
    if text.is_empty() {return None;}
    let mut closer = i32::MAX;
    let mut j = usize::MAX;
    for o in ntab {
	if text.eq(&addrs[o].nom_voie) && addrs[ind].nom_commune.eq(&addrs[o].nom_commune) {
	    if closer == i32::MAX {j=o;}
	    if let Some(n) = addrs[o].numero {
		if (n-num).abs()<closer {
		    j=o;
		    closer = (n-num).abs();
		    if closer== 0 {break;}
		}
	    }
	}
    }
    Some(j)
}

fn normalize_city(city:&str)->String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^st ").unwrap();
    }
    let mut c = city.to_lowercase();
    c.retain(|c| !r#"(),".;:'"#.contains(c));
    c = diacritics::remove_diacritics(&c);
    c = str::replace(&c,"-"," ");
    c = str::replace(&c," st "," saint ");
    c = RE.replace(&c,"saint ").into_owned();
    c
}

fn normalize_street(street:&str)->String {
    let mut s = street.to_lowercase();
    s.retain(|c| !r#"(),".;:'"#.contains(c));
    s=diacritics::remove_diacritics(&s);
    s = str::replace(&s,"-"," ");
    s
}

fn get_iris_adresses(r:&Patient,iris:&[Maille],addrs:&[Adresse]) -> Option<Upatient> {
    let cp = r.PST_CP.parse::<i32>().unwrap_or(0);
    let (num,street) = extract_info(&r.PST_ADRESSE);
    let city = normalize_city(&r.PST_VILLE);
//    println!("normalized: {:} {:} {:} {:}",num,street,cp,city);
    if let Some(j) = get_addrs(&street,num,cp,&city,addrs) {
	let s_ville = fuzzy_compare(&city,&addrs[j].nom_commune);
	let s_adresse = fuzzy_compare(&street,&addrs[j].nom_voie);
//	println!("{:?}",addrs[j]);
	let p = Point::new (addrs[j].lon,addrs[j].lat);
	for o in iris {
	    if o.geom.contains(&p) {
		let v = Upatient {
		    patient: r.N_PATIENT.clone(),
		    adresse: r.PST_ADRESSE.clone(),
		    cp: r.PST_CP.clone(),
		    ville: r.PST_VILLE.clone(),
		    n_adresse: addrs[j].nom_voie.clone(),
		    n_cp: addrs[j].code_postal,
		    n_ville: addrs[j].nom_commune.clone(),
		    iris: o.dcomiris.clone().unwrap(),
		    s_ville,
		    s_adresse
		};
		return Some(v);
	    }
	}
    }
    None
}


#[allow(dead_code)]
fn find_iris(filename:&str,iris:&[Maille],addrs:&[Adresse]) {
    let mut tab_ok = Vec::new();
    let mut tab_sok = Vec::new();
    let mut tab_nok = Vec::new();
    let csv = read_patients(filename);
    for o in csv {
	println!("{:?}",o);
	let now = Instant::now();
	let res = get_iris_adresses(&o,iris,addrs);
	println!("Searched for {:?}", now.elapsed());
	match res {
	    Some(v) => {
		println!("{:?}",v);
		if v.s_ville==1. && v.s_adresse==1. {tab_ok.push(v);}
		else {tab_sok.push(v);}
	    },
	    None => {
		println!("Address not found");
		tab_nok.push(o);
	    },
	}
    }
    write_patients("nok.csv",tab_nok);
    write_upatients("sok.csv",tab_sok);
    write_upatients("ok.csv",tab_ok);
}

fn clean_adresses(a:&mut [Adresse]){
    let now = Instant::now();
    for o in a {
	o.nom_voie = normalize_street(&o.nom_voie);
	o.nom_commune = normalize_city(&o.nom_commune);
    }
    println!("Addresses cleaned in {:?}", now.elapsed());
}

// Warning!!!!!!!!!!!!!!!!!!!
// The patient file must be in UTF-8 format
// Use: iconv -f ISO-8859-1 -t UTF-8 a.csv -o b.csv 

fn main() {
    let args:Vec<String> = std::env::args().collect();
    if args.len() != 2 {panic!("Wrong number of arguments");};
    
    let adresses = "adresses-france.csv";
    let my_adresses = "my-adresses.bin";
    let patient_file = &args[1];
    
    let meta = fs::metadata(adresses);
    let res = match meta {
	Ok(m) => {
	    let t1:u64 = m.created().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
	    let meta = fs::metadata(my_adresses);
	    match meta {
		Ok(m) => {
		    let t2:u64 = m.created().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
		    t2>t1
		},
		Err (_) => false
	    }
	},
	Err(_) => true
    };
	    
	
    let addrs = if ! res {
	println!("Need to rebuild addresses database");
	let mut addrs = read_adresses(adresses);
	clean_adresses(&mut addrs);
	let now = Instant::now();
	let file = File::create(my_adresses).unwrap();
	let mut writer = BufWriter::new(&file);
	bincode::serialize_into(&mut writer,&addrs).unwrap();
	println!("Addresses database written in {:?}",now.elapsed());
	addrs
    }
    else {
	let now = Instant::now();
	let file = File::open(my_adresses).unwrap();
	let mut reader = BufReader::new(&file);
	let addrs = bincode::deserialize_from(&mut reader).unwrap();
	println!("Addresses database read in {:?}",now.elapsed());
	addrs
    };

    let iris = read_iris();
    let now = Instant::now();
    find_iris(patient_file,&iris,&addrs);
    println!("Job done in {:?}",now.elapsed());
}



/*
    //    let osm = Openstreetmap::new();

fn get_iris_from_osm(buffer:&String,iris:&Vec<Maille>,osm:&Openstreetmap) -> Option<Maille> {
    let res = osm.forward(&buffer);
    let p:Vec<Point<f64>> = res.unwrap();
    if p.len()>0 {
	for i in 0..iris.len() {
	    if iris[i].geom.contains(&p[0]) {return Some(iris[i].clone());}
	}
    }
    return None;
}
*/

    /*
    if ! res {
	println!("Need to rebuild database");
	addrs = read_adresses(adresses);
	addrs = clean_adresses(addrs);
	let now = Instant::now();
	let file = File::create(my_adresses).unwrap();
	let mut wrt = WriterBuilder::new()
            .delimiter(b';')
            .from_writer(file);
	for i in 0..addrs.len() {
	    wrt.serialize(&addrs[i]).unwrap();
	}
	println!("Database written in {:?}",now.elapsed());
    }
    else {
	let file = File::open(my_adresses).unwrap();
	let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(file);
	let now = Instant::now();
	for result in rdr.deserialize() {
            let  record = result.unwrap();
	    addrs.push(record);
	}
	println!("Database read in {:?}",now.elapsed());
    }

    */
    /*
    // 396s
    // 1.4G
    {
    let now = Instant::now();
    let file_path = "adresses-france2.bin";
    let mut file = File::create(file_path).unwrap();
    rmp_serde::encode::write(&mut file,&addrs).unwrap();
    println!("{:?}",now.elapsed());
    }
    */

/*
    // 180s
    let file_path = "adresses-france2.bin";
    let mut file = File::open(file_path).unwrap();
    let now = Instant::now();
    let res = rmp_serde::decode::from_read(&mut file);
    println!("{:?}",now.elapsed());
    match res {
	Ok(addrs) => {
	    read_from_csv2(iris,addrs);
	},
	Err (_) => {}
    }
*/

    /*
    // 300s
    // 1.6G
    {
	let now = Instant::now();
	let file_path = "adresses-france2.bin";
	let mut file = File::create(file_path).unwrap();
	bincode::serialize_into(&mut file,&addrs).unwrap();
	println!("{:?}",now.elapsed());
    }
*/

  /*
    // 137s
    let now = Instant::now();
    let file_path = "adresses-france2.bin";
    let mut file = File::open(file_path).unwrap();
    let res = bincode::deserialize_from(&mut file);
    println!("{:?}",now.elapsed());
    match res {
	Ok(addrs) => {
	    read_from_csv2(iris,addrs);
	},
	Err (_) => {}
    }
*/
