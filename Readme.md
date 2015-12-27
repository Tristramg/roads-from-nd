# All roads from Notre-Dame

This projects contains the throw-away code used for the following
blogpost: http://blog.tristramg.eu/roads-from-notre-dame.html

To build the Rust part: `cargo build --release`

For the C++ tools, libpqxx and cairo (C bindings) are required:

* `g++ dump.cc --std=c++11 -lpq -lpqxx -O3 -o dump`
* `g++ draw.cc --std=c++11 -lcairo -O3 -o draw`

