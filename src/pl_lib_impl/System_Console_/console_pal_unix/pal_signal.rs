/// Translated from https://github.com/dotnet/runtime/blob/main/src/native/libs/System.Native/pal_signal.c
///
/* cSpell:disable */
use std::{
    ffi::c_void,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use libc::{PTHREAD_MUTEX_INITIALIZER, pid_t, pthread_mutex_t, sigaction, siginfo_t};

static mut lock: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;
static G_CONSOLE_TTOU_HANDLER: AtomicUsize = AtomicUsize::new(0);
pub static G_HANDLER_IS_INSTALLED: AtomicPtr<bool> = AtomicPtr::new(std::ptr::null_mut());
pub static G_ORIG_SIG_HANDLER: AtomicPtr<sigaction> = AtomicPtr::new(std::ptr::null_mut());
pub static mut G_SIGNAL_PIPE: [libc::c_int; 2] = [-1, -1];
pub static mut G_PID: pid_t = unsafe { std::mem::zeroed() };

pub static G_HAS_POSIX_SIGNAL_REGISTERATIONS: AtomicPtr<bool> =
    AtomicPtr::new(std::ptr::null_mut());

type ConsoleSigTtouHandler = extern "C" fn();

pub fn get_signal_max() -> libc::c_int {
    libc::SIGRTMAX()
}

pub fn close_signal_handling_pipe() {
    unsafe {
        assert!(G_SIGNAL_PIPE[0] >= 0);
        assert!(G_SIGNAL_PIPE[1] >= 0);
        libc::close(G_SIGNAL_PIPE[0]);
        libc::close(G_SIGNAL_PIPE[1]);
        G_SIGNAL_PIPE[0] = -1;
        G_SIGNAL_PIPE[1] = -1;
    }
}

pub fn install_ttou_handler_for_console(handler: ConsoleSigTtouHandler) {
    let installed;
    unsafe { libc::pthread_mutex_lock(&mut lock) };
    {
        assert!(G_CONSOLE_TTOU_HANDLER.load(Ordering::SeqCst) == 0);
        G_CONSOLE_TTOU_HANDLER.store(
            Box::leak(Box::new(handler)) as *mut ConsoleSigTtouHandler as usize,
            Ordering::SeqCst,
        );
        restore_signal_handler(libc::SIGTTOU);
        installed = unsafe { install_signal_handler(libc::SIGTTOU, libc::SA_RESETHAND) };
        assert!(installed);
    }
    unsafe { libc::pthread_mutex_unlock(&mut lock) };
}

pub fn uninstall_ttou_handler_for_console() {
    #[allow(unused)] // only used for assert
    let mut installed = false;

    unsafe { libc::pthread_mutex_lock(&mut lock) };
    {
        G_CONSOLE_TTOU_HANDLER.store(0, Ordering::SeqCst);

        restore_signal_handler(libc::SIGTTOU);
        if unsafe {
            *G_HAS_POSIX_SIGNAL_REGISTERATIONS
                .load(Ordering::SeqCst)
                .add((libc::SIGTTOU as usize) - 1)
        } {
            installed = unsafe { install_signal_handler(libc::SIGTTOU, libc::SA_RESTART) };
            assert!(installed);
        }
    }
    unsafe { libc::pthread_mutex_unlock(&mut lock) };
}

#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe fn install_signal_handler(sig: i32, flags: i32) -> bool {
    let mut rv;
    let orig = orig_action_for(sig);
    let is_installed = G_HANDLER_IS_INSTALLED
        .load(Ordering::SeqCst)
        .add((sig as usize) - 1);
    if *is_installed {
        return true;
    }
    rv = libc::sigaction(sig, std::ptr::null_mut(), orig);
    if rv != 0 {
        return false;
    }
    if is_sig_ign(&*orig) {
        *is_installed = true;
        return true;
    }
    let mut new_action;
    if !is_sig_dfl(&*orig) {
        new_action = *orig;
        new_action.sa_flags = (*orig).sa_flags & (!(libc::SA_RESTART | libc::SA_RESETHAND));
    } else {
        new_action = std::mem::zeroed();
    }
    new_action.sa_flags |= flags | libc::SA_SIGINFO;
    new_action.sa_sigaction = signal_handler as usize;
    rv = libc::sigaction(sig, &raw const new_action, orig);
    if rv != 0 {
        return false;
    }
    *is_installed = true;
    true
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn signal_handler(
    sig: libc::c_int,
    siginfo: *mut siginfo_t,
    context: *mut c_void,
) {
    if sig == libc::SIGCONT {
        let console_ttou_handler = std::mem::transmute::<_, ConsoleSigTtouHandler>(
            G_CONSOLE_TTOU_HANDLER.load(Ordering::SeqCst),
        );
        if console_ttou_handler as usize != 0 {
            console_ttou_handler();
        }
    }

    if !is_cancelable_termination_signal(sig) {
        let orig_handler = orig_action_for(sig);
        if (!is_sig_dfl(&*orig_handler)) && (!is_sig_ign(&*orig_handler)) {
            if is_sa_sig_info(&*orig_handler) {
                assert_ne!((&*orig_handler).sa_sigaction, 0);
                std::mem::transmute::<usize, extern "C" fn(libc::c_int, *mut siginfo_t, *mut c_void)>(
                    (&*orig_handler).sa_sigaction,
                )(sig, siginfo, context);
            } else {
                assert_ne!((&*orig_handler).sa_sigaction, 0);
                std::mem::transmute::<usize, extern "C" fn(libc::c_int)>(
                    (&*orig_handler).sa_sigaction,
                )(sig);
            }
        }
    }
    let signal_code_byte = sig as u8;
    let mut written_bytes = libc::write(G_SIGNAL_PIPE[1], (&raw const signal_code_byte).cast(), 1);
    while written_bytes < 0
        && std::io::Error::last_os_error().raw_os_error().unwrap() == libc::EINTR
    {
        written_bytes = libc::write(G_SIGNAL_PIPE[1], (&raw const signal_code_byte).cast(), 1);
    }

    if written_bytes != 1 {
        libc::abort();
    }
}

fn is_cancelable_termination_signal(sig: libc::c_int) -> bool {
    sig == libc::SIGINT || sig == libc::SIGQUIT || sig == libc::SIGTERM
}

fn is_sa_sig_info(action: &sigaction) -> bool {
    action.sa_flags & libc::SA_SIGINFO != 0
}

fn is_sig_ign(action: &sigaction) -> bool {
    (!is_sa_sig_info(action)) && action.sa_sigaction == libc::SIG_IGN
}

fn is_sig_dfl(action: &sigaction) -> bool {
    (!is_sa_sig_info(action)) && action.sa_sigaction == libc::SIG_DFL
}

fn restore_signal_handler(sig: i32) {
    unsafe {
        *G_HANDLER_IS_INSTALLED
            .load(Ordering::SeqCst)
            .add((sig as usize) - 1) = false;
        libc::sigaction(sig, orig_action_for(sig), std::ptr::null_mut());
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn orig_action_for(sig: i32) -> *mut sigaction {
    G_ORIG_SIG_HANDLER
        .load(Ordering::SeqCst)
        .add((sig as usize) - 1)
}
