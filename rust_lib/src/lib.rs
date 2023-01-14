mod java_glue;

use std::mem::MaybeUninit;
pub use crate::java_glue::*;

use android_logger::{Config, PlatformLogWriter};
use log::{Level, Record};
use rifgen::rifgen_attr::*;
use std::path::Path;
use std::{fmt, mem, ptr};
use std::ffi::{CStr, CString};
use file_mode::{ModePath, User};
use file_owner::PathExt;
use nix::errno::Errno;
use nix::unistd::{Gid, Uid, Group as NixGroup, SysconfVar, sysconf};
use nix::libc::{
    self, c_char, c_int, c_long, c_uint, c_void, gid_t, mode_t, off_t, pid_t,
    size_t, uid_t, PATH_MAX,
};

pub struct RustLog;

impl RustLog {
    //set up logging
    #[generate_interface]
    pub fn initialise_logging() {
        //#[cfg(target_os = "android")]
            android_logger::init_once(
            Config::default()
                .with_min_level(Level::Error)
                .with_tag("Rust"),
        );
        //log_panics::init();
        log::info!("Logging initialised from Rust");
        log::error!("Logging initialised from Rust");
    }
}

pub struct Inputs {
    argument: String,
}

impl Inputs {
    #[generate_interface(constructor)]
    pub fn new(argument: String) -> Inputs {
        Self {
            argument,
        }
    }
    #[generate_interface]
    pub fn mode(&self) -> String {
        let path = Path::new(&self.argument);
        let mode = path.mode().unwrap();
        mode.to_string()
    }
    #[generate_interface]
    pub fn owner(&self) -> String {
        let path = Path::new(&self.argument);
        path.owner().unwrap().to_string()
    }
    #[generate_interface]
    pub fn owner_id(&self) -> u32 {
        let path = Path::new(&self.argument);
        path.owner().unwrap().id()
    }
    #[generate_interface]
    pub fn group_id(&self) -> u32 {
        let path = Path::new(&self.argument);
        path.group().unwrap().id()
    }
    #[generate_interface]
    pub fn group(&self) -> String {
        let path = Path::new(&self.argument);
        //Inputs::loggg("group 0");
        let group = path.group();
        log::error!("group 1");
        log::error!("group is_ok {}", group.is_ok());
        match group {
            Ok(value) => {
                let id = value.id();
                let gid = Gid::from(value.id());
                /*let group = NixGroup::from_gid(gid);
                value.name().unwrap();*/
                let result = Inputs::from_anything(|grp, cbuf, cap, res| {
                    log::error!("group from_anything...");
                    unsafe { libc::getgrgid_r(id, grp, cbuf, cap, res) }
                });
                String::from("[ok]")
            }
            Err(e) => String::from("[err2]")
        }
        /*match group {
            Ok(value) => match value.name() {
                Ok(v) => match v {
                    Some(it) => it,
                    None => String::from("[null]")
                }
                Err(e) => String::from("[err1]")
            }
            Err(e) => String::from("[err2]")
        }*/
    }

    pub fn from_anything<F>(f: F) -> nix::Result<Option<NixGroup>>
        where
            F: Fn(*mut libc::group,
                *mut c_char,
                libc::size_t,
                *mut *mut libc::group) -> libc::c_int
    {
        log::error!("from_anything 0");
        let buflimit = 1048576;
        let bufsize = match sysconf(SysconfVar::GETGR_R_SIZE_MAX) {
            Ok(Some(n)) => n as usize,
            Ok(None) | Err(_) => 16384,
        };
        log::error!("from_anything 1");

        let mut cbuf = Vec::with_capacity(bufsize);
        let mut grp = MaybeUninit::<libc::group>::uninit();
        let mut res = ptr::null_mut();

        let mut c = 0;
        loop {
            log::error!("from_anything loop [{}]", c);
            c += 1;
            let error = f(grp.as_mut_ptr(), cbuf.as_mut_ptr(), cbuf.capacity(), &mut res);
            log::error!("from_anything loop 0, {}", error);
            if error == 0 {
                log::error!("from_anything loop 1, {}", res.is_null());
                if res.is_null() {
                    return Ok(None);
                } else {
                    log::error!("from_anything loop 4");
                    let grp = unsafe { grp.assume_init() };
                    log::error!("from_anything loop 5");
                    log::error!("from_anything loop 6 {}", grp.gr_gid);
                    log::error!("from_anything loop 7 {}", grp.gr_passwd.is_null());
                    unsafe {
                        log::error!("from_anything unsafe 00");
                        let name = CStr::from_ptr(grp.gr_name).to_string_lossy().into_owned();
                        log::error!("from_anything unsafe 1");
                        let pass = CStr::from_ptr(grp.gr_passwd);
                        log::error!("from_anything unsafe 2");
                        let passw = pass.to_bytes();
                        log::error!("from_anything unsafe 3");
                        let passwd = CString::new(passw);
                        log::error!("from_anything unsafe 4");
                        let passwd1 = passwd.unwrap();
                        log::error!("from_anything unsafe 5");
                        let passwd2 = passwd1.to_string_lossy().to_string();
                        log::error!("from_anything unsafe 6");
                        let gid = Gid::from_raw(grp.gr_gid);
                        log::error!("from_anything unsafe 7");
                        //let mem = NixGroup::members(gr.gr_mem);
                        log::error!("from_anything unsafe 8 {} {} {}", name, passwd2, gid);
                    }
                    return Ok(Some(NixGroup::from(&grp)));
                }
            } else if Errno::last() == Errno::ERANGE {
                log::error!("from_anything loop 2");
                // Trigger the internal buffer resizing logic.
                Inputs::reserve_double_buffer_size(&mut cbuf, buflimit)?;
            } else {
                log::error!("from_anything loop 3");
                return Err(Errno::last());
            }
        }
    }
    fn reserve_double_buffer_size<T>(buf: &mut Vec<T>, limit: usize) -> nix::Result<()> {
        use std::cmp::min;

        if buf.capacity() >= limit {
            return Err(Errno::ERANGE);
        }

        let capacity = min(buf.capacity() * 2, limit);
        buf.reserve(capacity);

        Ok(())
    }
}