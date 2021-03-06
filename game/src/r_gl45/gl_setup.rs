use fate::gx::{self, gl};
use platform::Platform;


static mut NB_ERRORS: usize = 0;

fn gl_debug_message_callback(msg: &gx::DebugMessage) {
    match ::std::ffi::CString::new(msg.text) {
        Ok(cstr) => debug!("GL: {}", cstr.to_string_lossy()),
        Err(e) => debug!("GL (UTF-8 error): {}", e),
    };
}

fn gl_post_hook(name: &str) {
    if name == "GetError" {
        return;
    }
    trace!("gl{}()", name);
    if unsafe { gx::SHOULD_TEMPORARILY_IGNORE_ERRORS } {
        return;
    }
    check_gl!(name);
}

fn gl_error_hook(e: Option<gx::Error>, s: &str) {
    match e {
        Some(e) => {
            error!("GL error: {:?} ({})", e, s);
            unsafe { NB_ERRORS += 1; }
        },
        None => if unsafe { NB_ERRORS > 0 } {
            panic!("Encountered {} OpenGL errors.", unsafe { NB_ERRORS });
        }
    }
}

pub fn gl_setup(platform: &Platform) {
    gl::load_with(|s| {
        let f = platform.gl_get_proc_address(s);
        trace!("GL: {}: {}", if f.is_null() { "Failed" } else { "Loaded" }, s);
        f
    });
    info!("OpenGL context summary:\n{}", gx::ContextSummary::new());
    gx::set_error_hook(gl_error_hook);
    unsafe { gl::POST_HOOK = gl_post_hook; }
    gx::boot_gl();
    gx::set_debug_message_callback(Some(gl_debug_message_callback));
    gx::log_debug_message("OpenGL debug logging is enabled.");

    let max_tex_units = gx::get::integer(gl::MAX_TEXTURE_IMAGE_UNITS); // NOTE: Min. 16
    info!("OpenGL max texture units: {}", max_tex_units);
}


