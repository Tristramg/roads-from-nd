extern crate byteorder;
extern crate postgres;
extern crate time;
extern crate pbr;
extern crate graphics;
extern crate image;


use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::io::Result as IoResult;
use std::{cmp, env};
use std::collections::HashMap;
use byteorder::{LittleEndian, ReadBytesExt};
use postgres::{Connection, SslMode};
use pbr::ProgressBar;
use image::ImageBuffer;

struct Node {
    osm_id: u32,
    lon: f32,
    lat: f32,
}

struct Edge {
    source: usize,
    target: usize,
    weight: u32,
}

struct Graph {
    edges: Vec<Edge>,
    nodes: Vec<Node>,
}

impl Node {
    fn from_osrm(reader: &mut BufReader<&File>) -> IoResult<Node> {
        let lat = try!(reader.read_i32::<LittleEndian>());
        let lon = try!(reader.read_i32::<LittleEndian>());
        let id = try!(reader.read_u32::<LittleEndian>());
        let _ = try!(reader.seek(SeekFrom::Current(4)));

        Ok(Node {
            osm_id: id,
            lon: lon as f32 / 1e6,
            lat: lat as f32 / 1e6
        })
    }
}

impl Edge {
    fn from_osrm(reader: &mut BufReader<&File>) -> IoResult<Edge> {
        let source = try!(reader.read_u32::<LittleEndian>()) as usize;
        let target = try!(reader.read_u32::<LittleEndian>()) as usize;
        let _ = try!(reader.seek(SeekFrom::Current(4)));
        let weight = try!(reader.read_u32::<LittleEndian>());
        let _ = try!(reader.seek(SeekFrom::Current(4)));

        Ok(Edge {
            source: source,
            target: target,
            weight: weight,
        })
    }
}

impl Graph {

    fn new(file: &String, source_osm_id: u32) -> IoResult<(Graph, usize)> {
        let file = try!(File::open(file)); 

        let mut reader = BufReader::new(&file);
        let mut source_index = 0;
        let _ = reader.seek(SeekFrom::Start(156));

        let nodes_count = try!(reader.read_u32::<LittleEndian>()) as usize;
        println!("  — Reading {:?} nodes", nodes_count);
        let mut nodes = Vec::with_capacity(nodes_count);
        let mut n_pb = ProgressBar::new(nodes_count);
        for i in 0..nodes_count {
            let node = try!(Node::from_osrm(&mut reader));
            if node.osm_id == source_osm_id {
                source_index = i
            }
            nodes.push(node);
            if i % 1000 == 0 {
                n_pb.add(1000);
            }
        }
        n_pb.add(nodes_count % 1000);

        let edges_count = try!(reader.read_u32::<LittleEndian>()) as usize;
        println!("  – Reading {:?} edges", edges_count);
        let mut edges = Vec::with_capacity(edges_count);
        let mut e_pb = ProgressBar::new(edges_count);
        for i in 0..edges_count {
            let edge = try!(Edge::from_osrm(&mut reader));
            edges.push(edge);
            if i % 1000 == 0 {
                e_pb.add(1000);
            }
        }
        e_pb.add(nodes_count % 1000);

        Ok((Graph { edges: edges, nodes: nodes }, source_index))
    }

    fn bellman(&self, source: usize) -> Vec<usize> {
        let nodes_count = self.nodes.len();
        let mut pred = (0..nodes_count).collect::<Vec<_>>();
        let mut dist = std::iter::repeat(std::u32::MAX).take(nodes_count).collect::<Vec<_>>();
        dist[source] = 0;

        let mut improvement = true;
        while improvement {
            improvement = false;
            for edge in &self.edges {
                let source_dist = dist[edge.source];
                let target_dist = dist[edge.target];
                if source_dist != std::u32::MAX && source_dist + edge.weight < target_dist {
                    dist[edge.target] = source_dist + edge.weight;
                    pred[edge.target] = edge.source;
                    improvement = true;
                }

                if target_dist != std::u32::MAX && target_dist + edge.weight < source_dist {
                    dist[edge.source] = target_dist + edge.weight;
                    pred[edge.source] = edge.target;
                    improvement = true;
                }
            }
        }

        pred
    }

    fn save_uses(&self, pred: Vec<usize>) {
        let mut edge_uses = HashMap::with_capacity(self.edges.len());

        println!("  — Counting the use of every edge");
        let mut cpb = ProgressBar::new(self.nodes.len());
        let mut count = 0;
        for node in 0..self.nodes.len() {
            let mut current_node = node;
            let mut pred_node = pred[current_node];
            while pred_node != current_node {
                let source = cmp::min(current_node, pred_node);
                let target = cmp::max(current_node, pred_node);

                let counter = edge_uses.entry((source, target)).or_insert(0);
                *counter += 1;

                current_node = pred_node;
                pred_node = pred[current_node]
            }
            count += 1;
            if count % 1000 == 0 {
                cpb.add(1000);
            }
        }

        let conn = Connection::connect("postgres://tristram:tristram@localhost/blood", &SslMode::None).unwrap();
        // CREATE TABLE edge_use ( count INTEGER )
        // SELECT AddGeometryColumn( 'edge_use', 'geom', 4326, 'LINESTRING', 2)

        println!("  — Inserting into DB");
        let mut pb = ProgressBar::new(edge_uses.len());
        let mut count = 0;
        let trans = conn.transaction().ok().expect("Unable to create a transaction");
        for (&(s,t), v) in edge_uses.iter() { 
            let source = &self.nodes[s];
            let target = &self.nodes[t];
            let geom = format!("st_GeomFromText('LINESTRING({} {}, {} {})', 4326)", source.lon, source.lat, target.lon, target.lat);
            trans.execute(&format!("INSERT INTO edge_use VALUES({}, {})", v, geom), &[]).ok().expect("Insert edge failed");         

            count += 1;
            if count % 1000 == 0 {
                pb.add(1000);
            }
        }
        trans.commit().unwrap();
    }
}


fn main2() {
    let args: Vec<_> = env::args().collect();
    let start = time::now();

    println!("Loading the data");
    let (graph, source_index) = Graph::new(&args[1], 0).unwrap();
    println!("   duration: {}s\n", (time::now() - start).num_seconds());

    let bellman_start = time::now();
    println!("Starting Bellman-Ford algorithm (progress is approximative)");
    let pred = graph.bellman(source_index);
    println!("   duration: {}s\n", (time::now() - bellman_start).num_seconds());

    let db_start = time::now();
    println!("Saving into the database");
    graph.save_uses(pred);
    println!("   duration: {}s\n", (time::now() - db_start).num_seconds());

    println!("Total duration: {}s", (time::now() - start).num_seconds());
}

fn main() {
    if false {
        main2();
    }
    let mut image = ImageBuffer::<image::Rgb<u8>>::new(100, 100);
    image.get_pixel_mut(5, 5).data = [255, 255, 255];
    image.save("output.png");

    let image   = graphics::Image::new().rect(graphics::rectangle::square(0.0, 0.0, 200.0));

//    let img = image::ImageBuffer.new(100, 100);
}
