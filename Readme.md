# All roads from Notre-Dame

This projects contains the throw-away code used for the following
blogpost: https://tristramg.eu/roads-from-notre/

Examples of generated images can be found at: https://cloud.tristramg.eu/index.php/s/nKTxqGmDXBNfSs2

Install Cargo http://doc.crates.io/.

To run `cargo run --release -- --help`.

* You need an OpenStreetMap dump in the PBF format
* Choose a starting node from OpenStreetMap. The node must be on the street network

The program will require about the same amout of memory as the size of the PBF
file.

The running time varies a lot depending on the data source.

On my laptop ( Intel(R) Core(TM) i7-6560U CPU @ 2.20GHz), I need about
20 minutes to one hour to process 1Gb of OpenStreetMap data.
