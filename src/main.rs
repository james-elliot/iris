use geocoding::{Openstreetmap, Forward, Point};
use geojson::{GeoJson};
use geo_types::Geometry;
use std::convert::TryInto;
use std::fs;
use geo::algorithm::contains::Contains;
use std::{thread, time, io};
use std::time::Instant;
use std::io::Write;
//use std::convert::TryFrom;
//use serde_json;
//use std::str::FromStr;


#[derive(Debug)]
struct Maille<'a> {
    geom : Geometry<f64>,
//    com : Option<&'a str>, // Code insee commune
    pop : Option<i64>, // Population
    dcomiris : Option<&'a str>, // Code iris
    rev_med : Option<f64>, // Revenu median
    tx_bac : Option<f64>, // Taux de bachelier
    tx_chom : Option<f64>, // Taux de chomage
    tx_ouvr : Option<f64>  // Taux d'ouvriers
}

fn main() {
    let now = Instant::now();
    let contents = fs::read_to_string("indice-de-defavorisation-sociale-fdep-par-iris.geojson").expect("Something went wrong reading the file");
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
//		println!("{:?}",f[i]);
//		let com = f[i].property("t1_com").map(|v| (v.as_str().unwrap()));
		let dcomiris = f[i].property("c_dcomiris").map(|v| (v.as_str().unwrap()));
		let txchom0 = f[i].property("t1_txchom0").map(|v| (v.as_f64().unwrap()));
		let txouvr0 = f[i].property("t1_txouvr0").map(|v| (v.as_f64().unwrap()));
		let p09_pop = f[i].property("t1_p09_pop").map(|v| (v.as_i64().unwrap()));
		let txbac09 = f[i].property("t1_txbac09").map(|v| (v.as_f64().unwrap()));
		let rev_med = f[i].property("t1_rev_med").map(|v| (v.as_f64().unwrap()));
		let geom : Geometry<f64> = f[i].geometry.clone().unwrap().try_into().unwrap();
//		println!("{:?} {:?} {:?} {:?} {:?}",txchom0,txouvr0,p09_pop,txbac09,rev_med);
//		println!("{:?}",geom);
		let t = Maille {
		    geom : geom,
//		    com : com,
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
	    let osm = Openstreetmap::new();
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
			    for i in 0..tab.len() {
				if tab[i].geom.contains(&p[0]) {
				    println!("found iris in {:?}", now.elapsed());
				    println!("{:?}",tab[i]);
				    break;
				}
			    }
			}
			else {
			    println!("Invalid address");
			}
			let tm = time::Duration::from_millis(1100);
			thread::sleep(tm);
		    },
		    _ => {
			panic!("Invalid string");
		    }
		}
	    }
        },
        _ => {
            panic!("Looking for a feature collection but didnt find one");
        }
    };
}
