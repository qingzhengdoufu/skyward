// SPDX-License-Identifier: Apache-2.0
#pragma once
#include <cstdint>

namespace skyward_browser_limits {
inline constexpr unsigned kProcesses = 96;
// RLIMIT_NPROC counts Linux tasks, including pthreads, across the dedicated UID.
// WebRTC needs ICE/transport/media threads in addition to WebKit's baseline.
inline constexpr unsigned kTasks = 256;
inline constexpr uint64_t kResidentBytes = 1536ULL << 20;
inline bool Within(unsigned processes, unsigned tasks, uint64_t bytes) {
  return processes <= kProcesses && tasks <= kTasks && bytes <= kResidentBytes;
}
}
