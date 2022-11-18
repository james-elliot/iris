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

#[derive(Debug, Deserialize, Clone)]
struct Record {
    N_PATIENT: String,
    PST_ADRESSE: String,
    PST_CP: String,
    PST_VILLE: String
}
use std::fs::File;
use csv::ReaderBuilder;
fn read_csv() -> Vec<Record> {
    let mut tab = Vec::new();
    let file_path = "toto.csv";
    let file = File::open(file_path).unwrap();
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(file);
    for result in rdr.deserialize() {
        let record: Record = result.unwrap();
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
		let res = osm.forward(&buffer);
		println!("osm request duration : {:?}", now.elapsed());
		let p:Vec<Point<f64>> = res.unwrap();
		if p.len()>0 {
		    println!("{:?}",p[0]);
		    let now = Instant::now();
		    for i in 0..iris.len() {
			if iris[i].geom.contains(&p[0]) {
			    println!("found iris in {:?}", now.elapsed());
			    println!("{:?}",iris[i]);
			    break;
			}
		    }
		}
		else {println!("Invalid address");}
		let tm = time::Duration::from_millis(1100);
		thread::sleep(tm);
	    },
	    _ => {panic!("Invalid string")}
	}
    }
}

fn main() {
    let iris = read_iris();
    let osm = Openstreetmap::new();
    read_from_stdin(iris,osm);
    //let csv = read_csv();
}
