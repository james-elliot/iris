use geocoding::{Openstreetmap, Forward, Point};
use geojson::{GeoJson};
use geo_types::Geometry;
use std::convert::TryInto;
use std::fs;
use geo::algorithm::contains::Contains;
use std::{thread, time};

//use std::convert::TryFrom;
//use serde_json;
//use std::str::FromStr;


#[derive(Debug)]
struct Maille {
    geom : Geometry<f64>,
    pop : Option<i64>, // Population
    rev_med : Option<f64>, // Revenu median
    tx_bac : Option<f64>, // Taux de bachelier
    tx_chom : Option<f64>, // Taux de chomage
    tx_ouvr : Option<f64>  // Taux d'ouvriers
}

fn main() {
    let contents = fs::read_to_string("indice-de-defavorisation-sociale-fdep-par-iris.geojson").expect("Something went wrong reading the file");
    let geojson = contents.parse::<GeoJson>().unwrap();

    let mut tab = Vec::new();
    match geojson {
        GeoJson::FeatureCollection(ctn) => {
	    let f = ctn.features;
            println!("Found {} features", f.len());
	    for i in 0..f.len() {
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
		    pop : p09_pop,
		    rev_med : rev_med,
		    tx_bac : txbac09,
		    tx_chom : txchom0,
		    tx_ouvr : txouvr0
		};
		tab.push(t);
	    }
	    let osm = Openstreetmap::new();
	    let address = "3 allee de la Bresse, Colomiers, France";
	    let res = osm.forward(&address);
	    let p:Vec<Point<f64>> = res.unwrap();
	    println!("{}",p.len());
	    println!("{:?} {:?}",p,p[0]);
	    for i in 0..tab.len() {
		if tab[i].geom.contains(&p[0]) {
		    println!("{:?}",tab[i]);
		}
	    }
	    let tm = time::Duration::from_millis(1100);
	    thread::sleep(tm);
        },
        _ => {
            panic!("Looking for a feature collection but didnt find one");
        }
    };
}