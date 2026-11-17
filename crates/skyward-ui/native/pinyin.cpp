// SPDX-License-Identifier: Apache-2.0
#include "pinyinime.h"
#include <cstring>
extern "C" size_t skyward_pinyin_query(const char *input, size_t index,
    unsigned short *output, size_t capacity, size_t *consumed) {
  using namespace ime_pinyin;
  if (!input || !output || capacity < 2 || !consumed) return 0;
  const size_t length = strlen(input);
  if (!length || length > 32) return 0;
  for (size_t i = 0; i < length; ++i)
    if ((input[i] < 'a' || input[i] > 'z') && input[i] != '\'') return 0;
  im_reset_search();
  const size_t count = im_search(input, length);
  if (index >= count || !im_get_candidate(index, output, capacity)) return count;
  const uint16 *starts = nullptr;
  const size_t syllables = im_get_spl_start_pos(starts);
  size_t chars = 0;
  while (chars < capacity && output[chars]) ++chars;
  *consumed = starts && chars <= syllables ? starts[chars] : length;
  if (*consumed > length) *consumed = length;
  return count;
}
