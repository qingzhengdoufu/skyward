// Linux-only reproduction of RLIMIT_NPROC counting pthreads, not just processes.
#include "resource_limits.hpp"
#include <cerrno>
#include <cstdio>
#include <pthread.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>

static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t condition = PTHREAD_COND_INITIALIZER;
static bool stopped;
static void *Wait(void *) {
  pthread_mutex_lock(&mutex);
  while (!stopped) pthread_cond_wait(&condition, &mutex);
  pthread_mutex_unlock(&mutex);
  return nullptr;
}
static int Run(unsigned limit, bool expect_success) {
  const pid_t child = fork();
  if (child < 0) return 1;
  if (!child) {
    rlimit bound{limit, limit};
    if (setrlimit(RLIMIT_NPROC, &bound) || setgid(99091) || setuid(99091)) _exit(2);
    pthread_t threads[128];
    unsigned made = 0;
    int error = 0;
    for (; made < 128; ++made) {
      error = pthread_create(&threads[made], nullptr, Wait, nullptr);
      if (error) break;
    }
    pthread_mutex_lock(&mutex);
    stopped = true;
    pthread_cond_broadcast(&condition);
    pthread_mutex_unlock(&mutex);
    for (unsigned i = 0; i < made; ++i) pthread_join(threads[i], nullptr);
    printf("TASK_LIMIT_TEST limit=%u created=%u error=%d\n", limit, made, error);
    fflush(stdout);
    _exit(expect_success ? (made == 128 ? 0 : 3) : (made < 96 && error == EAGAIN ? 0 : 4));
  }
  int status = 0;
  if (waitpid(child, &status, 0) != child) return 5;
  return WIFEXITED(status) ? WEXITSTATUS(status) : 6;
}
int main() {
  if (getuid()) return 77;
  const int previous = Run(96, false);
  return previous ? previous : Run(skyward_browser_limits::kTasks, true);
}
