// SPDX-License-Identifier: Apache-2.0
#pragma once
#include <cstddef>
#include <cstdint>

namespace skyward_frame {
inline bool Valid(int width, int height, uint32_t stride, size_t bytes) {
  if (width <= 0 || width > 4096 || height <= 0 || height > 4096) return false;
  const uint64_t row = uint64_t(width) * 4;
  if (stride < row || stride > row + 4096 || bytes > (64ULL << 20)) return false;
  return uint64_t(stride) * (height - 1) + row <= bytes;
}
inline bool SameAspect(int width, int height, int target_width, int target_height) {
  const int64_t difference = int64_t(width) * target_height - int64_t(height) * target_width;
  return difference >= -int64_t(target_width) * 4 && difference <= int64_t(target_width) * 4;
}
}
