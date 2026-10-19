#include <iostream>
#include <string>

#include "pidgeon_cat.pb.h"

int main() {
  pidgeon_cat::PidgeonCat cat;

  cat.set_id(42);
  cat.set_name("Pidgycat");

  std::string bytes;
  if (!cat.SerializeToString(&bytes)) {
    return 1;
  }

  pidgeon_cat::PidgeonCat restored;
  if (!restored.ParseFromString(bytes)) {
    return 1;
  }

  std::cout << restored.id() << ": " << restored.name() << std::endl;

  return 0;
}
