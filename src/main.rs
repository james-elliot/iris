use geocoding::{Openstreetmap, Forward, Point};
use geojson::{GeoJson};
use geo_types::Geometry;
use std::convert::TryInto;
use std::fs;
use geo::algorithm::contains::Contains;
use std::{thread, time, io};
use std::time::Instant;
use std::io::Write;
use serde::Deserialize;
use std::fs::File;
use csv::ReaderBuilder;

// Adresses au format AITF BAL 1.3 dans fichier csv
// Recuperables sur https://addresse.data.gouv.fr
// Contient environ 26 millions d'adresses
// 9Go nécessaires pour stocker le fichier en mémoire
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct Adresse {
//    uid_adresse:String,
//    cle_interop:String,
    commune_insee:String,
    commune_nom:String,
//    commune_deleguee_insee:String,
//    commune_deleguee_nom:String,
    voie_nom:String,
    lieudit_complement_nom:String,
    numero:String,
    suffixe:String,
//    position:String,
//    x:String,
//    y:String,
    long:f64,
    lat:f64,
//    cad_parcelles:String,
//    source:String,
//    date_der_maj:String,
//    certification_commune : String
}

#[allow(dead_code)]
fn read_adresses() -> Vec<Adresse> {
    let now = Instant::now();
    let mut tab = Vec::new();
    let file_path = "adresses-france.csv";
    let file = File::open(file_path).unwrap();
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(file);
    let mut nbi = 0;
    let mut nb = 0;
    for result in rdr.deserialize() {
	nb=nb+1;
	match result {
	    Ok(record) => {tab.push(record);},
//	    Err (_) =>{println!("{:?}",result);}
	    Err (_) =>{nbi=nbi+1;}
	}
    }
    println!("Addresses read and parsed in {:?}, {:} records, {:} invalid records", now.elapsed(),nb,nbi);
    return tab;
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct Adresse2 {
//    id:String,
//    id_fantoir:String,
    numero:String,
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

#[allow(dead_code)]
fn read_adresses2() -> Vec<Adresse2> {
    let now = Instant::now();
    let mut tab:Vec<Adresse2> = Vec::new();
    let file_path = "adresses-france2.csv";
    let file = File::open(file_path).unwrap();
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(file);
    let mut nbi = 0;
    let mut nb = 0;
    for result in rdr.deserialize() {
	nb=nb+1;
	match result {
	    Ok(r) => {
		tab.push(r);
	    },
//	    Err (_) =>{println!("{:?}",result);}
	    Err (_) =>{nbi=nbi+1;}
	}
    }
    println!("Addresses read and parsed in {:?}, {:} records, {:} invalid records", now.elapsed(),nb,nbi);
    let now = Instant::now();
    tab.sort_by(|a, b| a.code_postal.cmp(&b.code_postal));
    println!("Addresses sorted in {:?}", now.elapsed());
    for i in 0..tab.len()-1 {
	if tab[i].code_postal>tab[i+1].code_postal {
	    println!("{:?}\n{:?}\n",tab[i],tab[i+1]);
	}
    }
    return tab;
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code,non_snake_case)]
struct Patient {
    N_PATIENT: String,
    PST_ADRESSE: String,
    PST_CP: String,
    PST_VILLE: String
}
fn read_csv() -> Vec<Patient> {
    let mut tab = Vec::new();
    let file_path = "toto.csv";
    let file = File::open(file_path).unwrap();
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(file);
    for result in rdr.deserialize() {
        let record = result.unwrap();
	tab.push(record);
    }
    return tab;
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
	.expect("Something went wrong reading the file");
    println!("File read in {:?}", now.elapsed());
    let now = Instant::now();
    let geojson = contents.parse::<GeoJson>().unwrap();
    println!("File parsed in {:?}", now.elapsed());
    let mut tab = Vec::new();
    match geojson {
        GeoJson::FeatureCollection(ctn) => {
	    let f = ctn.features;
            println!("Found {} features", f.len());
	    let now = Instant::now();
	    for i in 0..f.len() {
		let dcomiris = f[i].property("c_dcomiris").map(|v| v.as_str().unwrap().to_string());
		let txchom0  = f[i].property("t1_txchom0").map(|v| v.as_f64().unwrap());
		let txouvr0  = f[i].property("t1_txouvr0").map(|v| v.as_f64().unwrap());
		let p09_pop  = f[i].property("t1_p09_pop").map(|v| v.as_i64().unwrap());
		let txbac09  = f[i].property("t1_txbac09").map(|v| v.as_f64().unwrap());
		let rev_med  = f[i].property("t1_rev_med").map(|v| v.as_f64().unwrap());
		let geom : Geometry<f64> = f[i].geometry.clone().unwrap().try_into().unwrap();
		let t = Maille {
		    geom : geom,
		    dcomiris : dcomiris,
		    pop : p09_pop,
		    rev_med : rev_med,
		    tx_bac : txbac09,
		    tx_chom : txchom0,
		    tx_ouvr : txouvr0
		};
		tab.push(t);
	    }
	    println!("Struct built in {:?}", now.elapsed());
	}
	_ => {
	    panic!("No collection");
	}
    }
    return tab;
}

fn get_iris(buffer:&String,iris:&Vec<Maille>,osm:&Openstreetmap) -> Option<Maille> {
    let res = osm.forward(&buffer);
    let p:Vec<Point<f64>> = res.unwrap();
    if p.len()>0 {
	for i in 0..iris.len() {
	    if iris[i].geom.contains(&p[0]) {return Some(iris[i].clone());}
	}
    }
    return None;
}

#[allow(dead_code)]
fn read_from_stdin(iris:Vec<Maille>,osm:Openstreetmap) -> () {
    let stdin = io::stdin();
    let mut buffer = String::new();
    loop {
	print!("Enter address:");
	io::stdout().flush().unwrap();
	buffer.clear();
	match stdin.read_line(&mut buffer) {
	    Ok(_) => {
		let now = Instant::now();
		let res = get_iris(&buffer,&iris,&osm);
		println!("osm address request duration : {:?}", now.elapsed());
		match res {
		    Some(v) => {println!("{:?}",v);},
		    None => {println!("Address not found");},
		}
		let tm = time::Duration::from_millis(1100);
		thread::sleep(tm);
	    },
	    _ => {panic!("Invalid string")}
	}
    }
}

fn build_address(r:&Patient)-> String {
    let v = ",";
    let mut buffer = String::new();
    buffer.push_str(&r.PST_ADRESSE);
    buffer.push_str(v);
    buffer.push_str(&r.PST_CP);
    buffer.push_str(v);
    buffer.push_str(&r.PST_VILLE);
    let res = buffer.to_lowercase();
    return res;
}

fn find_voies(v:&String) -> (String,usize) {
    let voies = [
	("rue ","rue"),
	("avenue ","avenue"),
	("boulevard ","boulevard"),
	("allees ","allees"),
	("blv ","boulevard"),
	("blvd ","boulevard"),
	("allee ","allees"),
	("av ","avenue"),
	("ch ","chemin"),
	("imp ","impasse"),
	("route ","route"),
	("impasse ","impasse"),
	("passage ","passage"),
	("chemin ","chemin"),
    ];
    for i in 0..voies.len() {
	let (a,b)=voies[i];
	match v.find(a) {
	    Some(i) => {
		let mut nv = v[0..i].to_owned();
		nv.push_str(b);
		nv.push_str(&v[i+a.len()-1..]);
		return (nv,i)
	    },
	    None => {}
	    }
	}
    return (v.clone(),0);
}

fn find_num(v: &String,e:usize) -> (i32,String) {
    for i in (0..e).rev() {
	let c = v.chars().nth(i).unwrap();
	if c.is_digit(10) {
	    for j in (0..i).rev() {
		let c = v.chars().nth(j).unwrap();
		if ! c.is_digit(10) {
		    let num=v[j+1..i+1].parse::<i32>().unwrap();
		    return (num,v[e..].to_owned());
		}
	    }
	    println!("i={:}",i);
	    let num=v[0..i+1].parse::<i32>().unwrap();
	    return (num,v[e..].to_owned());
	}
    }
    return (0,v.to_owned());
}

use regex::Regex;
fn extract_info(r:&String)-> (i32,String) {
    let cpt = ["b","bis","ter","t"];
    let re = Regex::new(r"[0-9]").unwrap();
    let re2 = Regex::new(r"[^0-9]").unwrap();
    let re3 = Regex::new(r"[^0123456789 ]").unwrap();
    let re4 = Regex::new(r"[ ]").unwrap();
    let mut v = r.clone();
    v.retain(|c| !r#"(),".;:'"#.contains(c));
    v = v.to_lowercase();
    v = diacritics::remove_diacritics(&v);
    let (v,i)=find_voies(&v); 
    println!("{:}",v);

    let (num,v) = find_num(&v,i);
    println!("num:{:} v:{:}",num,v);
    let m = re.find(&v);
    match m {
	Some(m) => {
	    let i = m.start();
	    if i==0 {
		let j = re2.find(&v).unwrap().start();
		let num = v[i..j].parse::<i32>().unwrap();
		let k = re3.find(&v).unwrap().start();
		let l = k+1+re4.find(&v[(k+1)..]).unwrap().start();
		let comp = &v[k..l];
		let street=
		    if cpt.contains(&comp) {
			let m=l+re3.find(&v[l..]).unwrap().start();
			&v[m..]
		    }
		else {
		    &v[k..]
		};
//		println!("num={:} cmp={:} street={:}",num,comp,street);
		return (num,street.to_string());
	    }
	},
	None=>{
	}
    }
    return (0,"".to_owned());
}


fn get_iris_adresses(r:&Patient,iris:&Vec<Maille>,addrs:&Vec<Adresse2>) -> Option<Maille> {
    let res = r.PST_CP.parse::<i32>();
    match res {
	Ok(cp) => {
	    let (num,street)=extract_info(&r.PST_ADRESSE);
	    let city = r.PST_VILLE.to_lowercase();
	    println!("{:} {:} {:} {:}",num,street,cp,city);
	},
	Err (_) => {println!("No CP");}
    }
    return None;
}

#[allow(dead_code)]
fn read_from_csv2(iris:Vec<Maille>,addrs:Vec<Adresse2>) -> () {
    let csv = read_csv();
    for i in 0..csv.len() {
	println!("{:?}",csv[i]);
	let now = Instant::now();
	let res = get_iris_adresses(&csv[i],&iris,&addrs);

	match res {
	    Some(v) => {println!("{:?}",v);},
	    None => {println!("Address not found");},
	}
    }
}

#[allow(dead_code)]
fn read_from_csv(iris:Vec<Maille>,osm:Openstreetmap) -> () {
    let csv = read_csv();
    for i in 0..csv.len() {
	println!("{:?}",csv[i]);
	let addr = build_address(&csv[i]);
	println!("{:?}",addr);
	let now = Instant::now();
	let res = get_iris(&addr,&iris,&osm);
	println!("osm address request duration : {:?}", now.elapsed());
	match res {
	    Some(v) => {println!("{:?}",v);},
	    None => {println!("Address not found");},
	}
	let tm = time::Duration::from_millis(1100);
	thread::sleep(tm);
    }
}

fn clean_adresses(mut a:Vec<Adresse2>) -> Vec<Adresse2> {
    let now = Instant::now();
    for i in 0..a.len() {
	a[i].nom_voie = a[i].nom_voie.to_lowercase();
	a[i].nom_voie.retain(|c| !r#"(),".;:'"#.contains(c));
	a[i].nom_voie = diacritics::remove_diacritics(&a[i].nom_voie);
	a[i].nom_commune = a[i].nom_commune.to_lowercase();
	a[i].nom_commune.retain(|c| !r#"(),".;:'"#.contains(c));
	a[i].nom_commune = diacritics::remove_diacritics(&a[i].nom_commune);
    }
    println!("Addresses cleaned in {:?}", now.elapsed());
    return a;
}

fn main() {
    let iris = read_iris();
    let osm = Openstreetmap::new();
    //    read_from_stdin(iris,osm);
    let mut addrs = read_adresses2();
    addrs=clean_adresses(addrs);
//    read_from_csv(iris,osm);
    read_from_csv2(iris,addrs);
}
