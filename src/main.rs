use geo::Contains;
use std::time::Instant;
use serde::{Serialize,Deserialize};
use std::fs::File;
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
    let mut rdr = csv::ReaderBuilder::new().delimiter(b';').from_reader(file);
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
    let mut rdr = csv::ReaderBuilder::new().delimiter(b';').from_reader(file);
    for result in rdr.deserialize() {tab.push(result.unwrap());}
    tab
}

fn write_patients(file_path: &str,v:Vec<Patient>) {
    let file = File::create(file_path).unwrap();
    let mut wrt = csv::WriterBuilder::new().delimiter(b';').from_writer(file);
    for o in v {wrt.serialize(&o).unwrap();}
    wrt.flush().unwrap();
}

fn write_upatients(file_path: &str,v:Vec<Upatient>) {
    let file = File::create(file_path).unwrap();
    let mut wrt = csv::WriterBuilder::new().delimiter(b';').from_writer(file);
    for o in v {wrt.serialize(&o).unwrap();}
    wrt.flush().unwrap();
}

#[derive(Debug,Clone)]
struct Maille {
    geom     : geo::Geometry<f64>,
    dcomiris : Option<String>, // Code iris
}

fn convert(c:&geo_types::Coord) -> geo_types::Coord {
    let point = lambert::Point::new(c.x as f32, c.y as f32, 0.0)
        .wgs84_from_meter(lambert::Zone::Lambert93)
        .convert_unit(lambert::AngleUnit::Radian, lambert::AngleUnit::Degree);
    geo_types::Coord{x:point.x as f64,y:point.y as f64}
}

#[allow(dead_code)]
fn read_iris() -> Vec<Maille> {
    let now = Instant::now();
    let mut tab = Vec::new();
    let mut reader = shapefile::Reader::from_path("CONTOURS/CONTOURS-IRIS.shp").unwrap();
    for shape_record in reader.iter_shapes_and_records() {
	let (shape, record) = shape_record.unwrap();
	 if let shapefile::record::Shape::Polygon(pl) = shape {
	     let p: geo_types::MultiPolygon<f64> = pl.into();
	     let p2: geo_types::Polygon<f64> = p.iter().next().unwrap().clone();
	     //		println!("{:?}", p2);
	     let l = p2.exterior();
	     let l2:Vec<geo_types::Coord> = l.coords().map(convert).collect();
	     let l3 = geo_types::LineString::new(l2);
	     let p3 = geo_types::Polygon::new(l3,vec![]);
	     let geom:geo_types::Geometry = p3.into();
	     //		println!("{:?}", geom);
	     for (name, value) in record {
		 if name.eq("CODE_IRIS") {
		     if let shapefile::dbase::FieldValue::Character(dcomiris) = value {
			 //				println! ("{:?}: {:?} ", geom, v);
			 tab.push(Maille {geom,dcomiris});
			 break;
		     }
		 }
	     }
	 }
    }
    println!("Iris database built in {:?}", now.elapsed());
    tab
}

#[allow(dead_code)]
fn read_iris2() -> Vec<Maille> {
    let now = Instant::now();
    let contents = std::fs::read_to_string("iris.geojson")
	.unwrap();
    let geojson = contents.parse::<geojson::GeoJson>().unwrap();
    let mut tab = Vec::new();
    match geojson {
        geojson::GeoJson::FeatureCollection(ctn) => {
	    let f = ctn.features;
            println!("Found {} features", f.len());
	    for a in &f {
		let dcomiris = a.property("c_dcomiris").map(|v| v.as_str().unwrap().to_string());
		let geom : geo::Geometry<f64> = a.geometry.as_ref().unwrap().try_into().unwrap();
		tab.push(Maille {geom,dcomiris});
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
		t1.push(Regex::new(a.0).unwrap());
		t2.push(Regex::new(&a.0[2..]).unwrap());
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

fn get_addrs(street:&str,num:i32,cp:i32,city:&str,addrs:&[Adresse])->Option<usize> {
    let mut text = String::new();
    let mut ind ;
    let mut tab = Vec::new();
    let mut ntab = Vec::new();
    let mut f = 1;
    loop {
	if find_first_last_cp(addrs,cp,f,&mut tab)  {
	    ind = *tab.iter().max_by_key(
		|x| (100.0*fuzzy_compare(city,&addrs[**x].nom_commune)) as i64
	    ).unwrap();
	    if fuzzy_compare(city,&addrs[ind].nom_commune) > 0.8 {
		let mut corpus = CorpusBuilder::new().arity(2).pad_full(Pad::Auto).finish();
		for o in &tab {
		    if addrs[ind].nom_commune.eq(&addrs[*o].nom_commune) {
			corpus.add_text(&addrs[*o].nom_voie);
			ntab.push(*o);
		    }
		}
		if let Some(t)=corpus.search(street, 0.8).first() {text.push_str(&t.text);break;}
	    }
	}
	if f==1000 {return None;}
	f = 1000;
	ntab.clear();
    }
    Some(
	*ntab.iter().min_by_key(
	    |x| {
		if text.eq(&addrs[**x].nom_voie) && addrs[ind].nom_commune.eq(&addrs[**x].nom_commune)
		{(addrs[**x].numero.unwrap_or(i32::MAX)-num).abs()}
		else {i32::MAX-num}
	    }
	).unwrap()
    )
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
    RE.replace(&c,"saint ").into_owned()
}

fn normalize_street(street:&str)->String {
    let mut s = street.to_lowercase();
    s.retain(|c| !r#"(),".;:'"#.contains(c));
    s=diacritics::remove_diacritics(&s);
    str::replace(&s,"-"," ")
}

fn get_iris_adresses(r:&Patient,iris:&[Maille],addrs:&[Adresse]) -> Option<Upatient> {
    let cp = r.PST_CP.parse::<i32>().unwrap_or(0);
    let (num,street) = extract_info(&r.PST_ADRESSE);
    let city = normalize_city(&r.PST_VILLE);
    println!("normalized: {:} {:} {:} {:}",num,street,cp,city);
    if let Some(j) = get_addrs(&street,num,cp,&city,addrs) {
	let s_ville = fuzzy_compare(&city,&addrs[j].nom_commune);
	let s_adresse = fuzzy_compare(&street,&addrs[j].nom_voie);
//	println!("{:?}",addrs[j]);
	let p = geo_types::Point::new (addrs[j].lon,addrs[j].lat);
	for o in iris {
	    if o.geom.contains(&p) {
		let mut addr = addrs[j].nom_voie.clone();
		if num !=0 {addr = num.to_string() + " " + &addr;}
		let v = Upatient {
		    patient: r.N_PATIENT.clone(),
		    adresse: r.PST_ADRESSE.clone(),
		    cp: r.PST_CP.clone(),
		    ville: r.PST_VILLE.clone(),
		    n_adresse: addr,
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
    
    let meta = std::fs::metadata(adresses);
    let res = match meta {
	Ok(m) => {
	    let ot1 = m.modified().unwrap();
	    let meta = std::fs::metadata(my_adresses);
	    match meta {
		Ok(m) => {
		    let ot2 = m.modified().unwrap();
		    ot2>ot1
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
	let mut writer = std::io::BufWriter::new(&file);
	bincode::serialize_into(&mut writer,&addrs).unwrap();
	println!("Addresses database written in {:?}",now.elapsed());
	addrs
    }
    else {
	let now = Instant::now();
	let file = File::open(my_adresses).unwrap();
	let mut reader = std::io::BufReader::new(&file);
	let addrs = bincode::deserialize_from(&mut reader).unwrap();
	println!("Addresses database read in {:?}",now.elapsed());
	addrs
    };

    let iris = read_iris();
    let now = Instant::now();
    find_iris(patient_file,&iris,&addrs);
    println!("Job done in {:?}",now.elapsed());
}


