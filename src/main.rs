extern crate osm4routing;
extern crate pdf;
extern crate docopt;

use docopt::Docopt;
use pdf::graphicsstate::Color;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::max;
use std::{i64, f64, f32};
use std::str::FromStr;
use std::time::SystemTime;

fn main() {
    const USAGE: &'static str = "Roads from Notre-Dame (or anywhere else)

Usage:
  roads-from-nd [options] <source.osm.pbf> <node> <output.pdf>
  roads-from-nd -h | --help

Options:
  -h --help      Show this screen
  --width WIDTH  Width of the largest stoke [default: 3]
  --keep EDGES   Keep only the NUM_EDGES most important edges [default: 100000]";

    let args = Docopt::new(USAGE).unwrap().parse().unwrap_or_else(|e| e.exit());

    let start_node = i64::from_str(args.get_str("<node>")).expect("Read source osm node");
    let max_width = f32::from_str(args.get_str("--width")).unwrap_or(3.);
    let keep_edges = usize::from_str(args.get_str("--keep")).unwrap_or(100000);

    let now = SystemTime::now();
    println!("Loading the graph");
    let g = Graph::from_osm(args.get_str("<source.osm.pbf>"));
    println!(" ✓ duration: {}s\n", now.elapsed().unwrap().as_secs());

    let now = SystemTime::now();
    println!("Running Dijkstra’s algorithm");
    let pred = g.dijkstra(start_node);
    println!(" ✓ duration: {}s\n", now.elapsed().unwrap().as_secs());

    let now = SystemTime::now();
    println!("Counting the use of each edge");
    let usages = g.count_uses(&pred);
    println!(" ✓ duration: {}s\n", now.elapsed().unwrap().as_secs());

    let now = SystemTime::now();
    println!("Rendering the PDF file");
    let bounds = g.bounds(&pred);
    g.render(&usages,
             args.get_str("<output.pdf>"),
             max_width,
             keep_edges,
             bounds);
    println!(" ✓ duration: {}s\n", now.elapsed().unwrap().as_secs());
}

struct Graph {
    adj_list: Vec<Vec<(usize, f64)>>,
    nodes: Vec<osm4routing::models::Node>,
    nodes_to_vertex: HashMap<i64, usize>,
}

impl Graph {
    fn from_osm(filename: &str) -> Graph {
        let (nodes, edges) = osm4routing::reader::read(filename).expect("Read OSM file");

        let mut nodes_to_vertex = HashMap::new();
        nodes_to_vertex.reserve(nodes.len());
        let mut adj_list = Vec::with_capacity(nodes.len());

        for (i, n) in nodes.iter().enumerate() {
            nodes_to_vertex.insert(n.id, i);
            adj_list.push(Vec::new());
        }

        for edge in edges.iter()
            .filter(|&e| e.properties.car_forward >= 2 || e.properties.car_backward >= 2) {
            let s = nodes_to_vertex[&edge.source];
            let t = nodes_to_vertex[&edge.target];
            if edge.properties.car_forward >= 2 {
                adj_list[s].push((t, edge.length() / (edge.properties.car_forward as f64)));
            }
            if edge.properties.car_backward >= 2 {
                adj_list[t].push((s, edge.length() / (edge.properties.car_backward as f64)));
            }
        }

        Graph {
            adj_list: adj_list,
            nodes: nodes,
            nodes_to_vertex: nodes_to_vertex,
        }
    }

    fn dijkstra(&self, source: i64) -> Vec<usize> {
        let mut pred = Vec::with_capacity(self.nodes.len());
        let mut dist = Vec::with_capacity(self.nodes.len());

        for i in 0..self.nodes.len() {
            pred.push(i);
            dist.push(f64::INFINITY);
        }

        let mut q = BinaryHeap::new();
        let start = self.nodes_to_vertex[&source];
        dist[start] = 0.;
        q.push(start);
        while !q.is_empty() {
            let v = q.pop().unwrap();
            for &(target, weigth) in &self.adj_list[v] {
                let new_weigth = dist[v] + weigth;
                if new_weigth < dist[target] {
                    pred[target] = v;
                    dist[target] = new_weigth;
                    q.push(target);
                }
            }
        }
        pred
    }

    fn count_uses(&self, pred: &Vec<usize>) -> Vec<((usize, usize), i32)> {
        let mut usages = HashMap::new();
        for destination in 0..self.nodes.len() {
            let mut v = destination;
            while v != pred[v] {
                let usage = usages.entry((pred[v], v)).or_insert(0);
                *usage += 1;
                v = pred[v];
            }
        }
        let mut as_vec: Vec<((usize, usize), i32)> = usages.into_iter().collect();
        as_vec.sort_by(|&(_, a), &(_, b)| a.cmp(&b));
        as_vec
    }

    fn bounds(&self, pred: &Vec<usize>) -> (f64, f64, f64, f64) {
        pred.iter().filter(|&u| pred[*u] != *u).fold((f64::MAX, f64::MIN, f64::MAX, f64::MIN),
                                                     |(xmin, xmax, ymin, ymax), u| {
            let c = self.nodes[*u].coord;
            (xmin.min(c.lon), xmax.max(c.lon), ymin.min(c.lat), ymax.max(c.lat))
        })
    }

    fn render(&self,
              uses: &Vec<((usize, usize), i32)>,
              filename: &str,
              max_width: f32,
              keep: usize,
              bounds: (f64, f64, f64, f64)) {
        let (lon_min, lon_max, y_min, y_max) = bounds;
        let avg_lat = y_min + (y_max - y_min) / 2.;
        let (x_min, x_max) = (stupid_proj(lon_min, avg_lat), stupid_proj(lon_max, avg_lat));

        let width = 1000.;
        let ratio = width / (x_max - x_min);
        let height = ratio * (y_max - y_min);

        let max_use = uses.iter().fold(0, |acc, &(_, c)| max(acc, c));

        let mut document = pdf::Pdf::create(filename).expect("Create pdf file");
        document.render_page(width as f32, height as f32, |canvas| {
                try!(canvas.set_line_cap_style(pdf::graphicsstate::CapStyle::Round));

                let skip = if uses.len() < keep {
                    0
                } else {
                    uses.len() - keep
                };
                println!("skip {}", skip);
                for &((u, v), count) in uses.iter().skip(skip) {
                    let width = sigma(max_use as f32, count as f32);
                    let c = (128. * (1. - width)).round() as u8;
                    try!(canvas.set_stroke_color(Color::rgb(c, c, c)));
                    try!(canvas.set_line_width(width * max_width));;

                    let x_a = stupid_proj(self.nodes[u].coord.lon, avg_lat);
                    let y_a = self.nodes[u].coord.lat;
                    let x_b = stupid_proj(self.nodes[v].coord.lon, avg_lat);
                    let y_b = self.nodes[v].coord.lat;
                    try!(canvas.line(((x_a - x_min) * ratio) as f32,
                                     ((y_a - y_min) * ratio) as f32,
                                     ((x_b - x_min) * ratio) as f32,
                                     ((y_b - y_min) * ratio) as f32));
                    try!(canvas.stroke());
                }
                Ok(())
            })
            .expect("Render the document");
        document.finish().expect("Finish pdf document");
    }
}

fn stupid_proj(lon: f64, lat: f64) -> f64 {
    lon * lat.to_radians().cos()
}

fn sigma(max: f32, x: f32) -> f32 {
    x.log(2.) / max.log(2.)
}

#[test]
fn compare_inf() {
    assert!(32. < f64::INFINITY);
}
