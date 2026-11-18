#include "resource_limits.hpp"
#include <cassert>
using namespace skyward_browser_limits;
int main() {
  assert(Within(3, 128, 768ULL << 20));
  assert(Within(kProcesses, kTasks, kResidentBytes));
  assert(!Within(kProcesses + 1, 10, 0));
  assert(!Within(3, kTasks + 1, 0));
  assert(!Within(3, 100, kResidentBytes + 1));
}
