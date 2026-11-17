// SPDX-License-Identifier: Apache-2.0
#include "frame_layout.hpp"
#include <cassert>
int main() {
  using skyward_frame::Valid;
  assert(Valid(1440, 2440, 5760, 1440ULL * 2440 * 4));
  assert(Valid(1440, 2440, 5888, 5888ULL * 2439 + 5760));
  assert(!Valid(1440, 2440, 5888, 5888ULL * 2439 + 5759));
  assert(!Valid(1440, 2440, 5759, 1440ULL * 2440 * 4));
  assert(!Valid(0, 2440, 5760, 1440ULL * 2440 * 4));
  assert(!Valid(1440, 4097, 5760, 1440ULL * 4097 * 4));
  assert(skyward_frame::SameAspect(360, 610, 1440, 2440));
  assert(!skyward_frame::SameAspect(1080, 2100, 1440, 2440));
}
