/* Post-fork helper supervisor: no allocation, stdio, Objective-C or pathname IO.
 * The helper leads a private group. Its PID remains waitable (WNOWAIT) until
 * the group has been terminated, preventing a reused PID/group from being hit.
 */
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <signal.h>
#include <errno.h>
#include <fcntl.h>
#include <time.h>

static volatile sig_atomic_t sh_backup_cancelled = 0;
static void sh_backup_stop(int signal_number) {
    (void)signal_number;
    sh_backup_cancelled = 1;
}
static void sh_backup_pause(void) {
    const struct timespec delay = {0, 50000000};
    (void)nanosleep(&delay, 0);
}
static int sh_backup_fd(int value) {
    if (value >= 3) return value;
    const int copy = fcntl(value, F_DUPFD_CLOEXEC, 3);
    const int saved = errno;
    (void)close(value);
    errno = saved;
    return copy;
}
static void sh_backup_finish(pid_t child, int group_known, int cancelled) {
    if (group_known || getpgid(child) == child) (void)kill(-child, SIGKILL);
    else (void)kill(child, SIGKILL);
    int status = 0;
    pid_t waited;
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    if (cancelled || waited != child) _exit(125);
    if (WIFEXITED(status)) _exit(WEXITSTATUS(status));
    if (WIFSIGNALED(status)) {
        const int number = WTERMSIG(status);
        struct sigaction action = {0};
        action.sa_handler = SIG_DFL;
        (void)sigemptyset(&action.sa_mask);
        (void)sigaction(number, &action, 0);
        (void)kill(getpid(), number);
        _exit(128 + number);
    }
    _exit(125);
}

/* Called by std::process::Command::pre_exec. Returns only in the actual helper
 * child (0) or before a helper exists (errno). The supervisor always _exit's.
 */
int stillharbour_backup_parent_guard(pid_t original_parent, int max_fd, int completion_fd) {
    if (original_parent <= 1 || max_fd < 8 || completion_fd < -1 ||
        (completion_fd >= 0 && (completion_fd < 3 || completion_fd >= max_fd))) return EINVAL;
    if (getppid() != original_parent) _exit(125);
    int ready[2];
    if (pipe(ready) != 0) return errno;
    ready[0] = sh_backup_fd(ready[0]);
    ready[1] = sh_backup_fd(ready[1]);
    if (ready[0] < 0 || ready[1] < 0) {
        const int saved = errno;
        if (ready[0] >= 0) (void)close(ready[0]);
        if (ready[1] >= 0) (void)close(ready[1]);
        return saved;
    }
    if (fcntl(ready[0], F_SETFD, FD_CLOEXEC) != 0 ||
        fcntl(ready[1], F_SETFD, FD_CLOEXEC) != 0 ||
        fcntl(ready[0], F_SETFL, O_NONBLOCK) != 0) {
        const int saved = errno; (void)close(ready[0]); (void)close(ready[1]); return saved;
    }
    sh_backup_cancelled = 0;
    struct sigaction action = {0};
    action.sa_handler = sh_backup_stop;
    (void)sigemptyset(&action.sa_mask);
    if (sigaction(SIGTERM, &action, 0) != 0 || sigaction(SIGINT, &action, 0) != 0 ||
        sigaction(SIGHUP, &action, 0) != 0) {
        const int saved = errno; (void)close(ready[0]); (void)close(ready[1]); return saved;
    }
    action.sa_handler = SIG_DFL;
    (void)sigaction(SIGCHLD, &action, 0);
    sigset_t unblocked;
    (void)sigemptyset(&unblocked);
    (void)sigaddset(&unblocked, SIGTERM); (void)sigaddset(&unblocked, SIGINT);
    (void)sigaddset(&unblocked, SIGHUP); (void)sigaddset(&unblocked, SIGCHLD);
    (void)sigprocmask(SIG_UNBLOCK, &unblocked, 0);
    const pid_t child = fork();
    if (child < 0) {
        const int saved = errno; (void)close(ready[0]); (void)close(ready[1]); return saved;
    }
    if (child == 0) {
        (void)close(ready[0]);
        if (setpgid(0, 0) != 0) { const int saved = errno; (void)close(ready[1]); return saved; }
        action.sa_handler = SIG_DFL;
        (void)sigaction(SIGTERM, &action, 0); (void)sigaction(SIGINT, &action, 0);
        (void)sigaction(SIGHUP, &action, 0);
        const char byte = 1;
        ssize_t count;
        do { count = write(ready[1], &byte, 1); } while (count < 0 && errno == EINTR);
        (void)close(ready[1]);
        if (count != 1) _exit(126);
        return 0;
    }
    (void)close(ready[1]);
    /* Only the supervisor closes these. The actual helper must retain std's
     * exec-error pipe until exec (or error propagation) and its intended stdio.
     */
    /* A private registration socket stays open in this guardian until its
     * actual child/group is stopped AND reaped. It is CLOEXEC in the helper.
     * Killing the Rust wrapper therefore cannot falsely report completion.
     */
    for (int fd = 3; fd < max_fd; ++fd)
        if (fd != ready[0] && fd != completion_fd) (void)close(fd);
    int group_known = 0;
    for (;;) {
        char byte = 0;
        const ssize_t count = read(ready[0], &byte, 1);
        if (count == 1 && byte == 1) group_known = 1;
        siginfo_t info = {0};
        const int observed = waitid(P_PID, (id_t)child, &info, WEXITED | WNOHANG | WNOWAIT);
        if (sh_backup_cancelled || getppid() != original_parent)
            sh_backup_finish(child, group_known, 1);
        if (observed == 0 && info.si_pid == child)
            sh_backup_finish(child, group_known, 0);
        if (observed != 0 && errno != EINTR)
            sh_backup_finish(child, group_known, 1);
        sh_backup_pause();
    }
}
