#include <pqxx/pqxx> 
#include <iostream>
#include <fstream>

// g++ dump.cc  --std=c++11 -lpq -lpqxx -O3 -o dump
struct edge {
  float x1, y1, x2, y2, count;

  edge(pqxx::result::tuple row) :
    x1(row[0].as<float>()),
    y1(row[1].as<float>()),
    x2(row[2].as<float>()),
    y2(row[3].as<float>()),
    count(row[4].as<float>())
  {}
};

int main() {
  std::ofstream out("edges_dump");
  pqxx::connection conn("dbname=blood user=tristram password=tristram");
  
  pqxx::work work(conn);

  pqxx::stateless_cursor<pqxx::cursor_base::read_only, pqxx::cursor_base::owned> c(work,
      "SELECT\
        st_x(st_pointn(geom, 1)),\
        st_y(st_pointn(geom, 1)),\
        st_x(st_pointn(geom, 2)),\
        st_y(st_pointn(geom, 2)),\
        count\
      FROM france",
      "cursor", false);
   

  size_t size = c.size();
  std::cout << "Size: " << size << std::endl;
  out.write(reinterpret_cast<const char *>(&size), sizeof(size));
  int count = 0;
  for ( size_t idx = 0; idx < size ; idx += 100000 )
  {
    pqxx::result result = c.retrieve( idx, std::min(idx + 100000, size)  );
    if ( result.empty() )
    {
        // nothing left to read
        break;
    }

    for(auto row : result) {
      edge e(row);
      out.write(reinterpret_cast<const char *>(&e), sizeof(e));
      count++;
      if(count % 10000 == 0) {
        std::cout << "." << std::flush;
      }
    }
  }
}

