// A module for searching for libraries
// FIXME (#2658): I'm not happy how this module turned out. Should
// probably just be folded into cstore.

use result::Result;
export filesearch;
export mk_filesearch;
export pick;
export pick_file;
export search;
export relative_target_lib_path;
export get_cargo_sysroot;
export get_cargo_root;
export get_cargo_root_nearest;
export libdir;

type pick<T> = fn(path: &Path) -> Option<T>;

fn pick_file(file: Path, path: &Path) -> Option<Path> {
    if path.file_path() == file { option::Some(copy *path) }
    else { option::None }
}

trait filesearch {
    fn sysroot() -> Path;
    fn lib_search_paths() -> ~[Path];
    fn get_target_lib_path() -> Path;
    fn get_target_lib_file_path(file: &Path) -> Path;
}

fn mk_filesearch(maybe_sysroot: Option<Path>,
                 target_triple: &str,
                 addl_lib_search_paths: ~[Path]) -> filesearch {
    type filesearch_impl = {sysroot: Path,
                            addl_lib_search_paths: ~[Path],
                            target_triple: ~str};
    impl filesearch_impl: filesearch {
        fn sysroot() -> Path { self.sysroot }
        fn lib_search_paths() -> ~[Path] {
            let mut paths = self.addl_lib_search_paths;

            vec::push(paths,
                      make_target_lib_path(&self.sysroot,
                                           self.target_triple));
            match get_cargo_lib_path_nearest() {
              result::Ok(p) => vec::push(paths, p),
              result::Err(_) => ()
            }
            match get_cargo_lib_path() {
              result::Ok(p) => vec::push(paths, p),
              result::Err(_) => ()
            }
            paths
        }
        fn get_target_lib_path() -> Path {
            make_target_lib_path(&self.sysroot, self.target_triple)
        }
        fn get_target_lib_file_path(file: &Path) -> Path {
            self.get_target_lib_path().push_rel(file)
        }
    }

    let sysroot = get_sysroot(maybe_sysroot);
    debug!("using sysroot = %s", sysroot.to_str());
    {sysroot: sysroot,
     addl_lib_search_paths: addl_lib_search_paths,
     target_triple: str::from_slice(target_triple)} as filesearch
}

fn search<T: Copy>(filesearch: filesearch, pick: pick<T>) -> Option<T> {
    let mut rslt = None;
    for filesearch.lib_search_paths().each |lib_search_path| {
        debug!("searching %s", lib_search_path.to_str());
        for os::list_dir_path(lib_search_path).each |path| {
            debug!("testing %s", path.to_str());
            let maybe_picked = pick(*path);
            if maybe_picked.is_some() {
                debug!("picked %s", path.to_str());
                rslt = maybe_picked;
                break;
            } else {
                debug!("rejected %s", path.to_str());
            }
        }
        if rslt.is_some() { break; }
    }
    return rslt;
}

fn relative_target_lib_path(target_triple: &str) -> Path {
    Path(libdir()).push_many([~"rustc",
                              str::from_slice(target_triple),
                              libdir()])
}

fn make_target_lib_path(sysroot: &Path,
                        target_triple: &str) -> Path {
    sysroot.push_rel(&relative_target_lib_path(target_triple))
}

fn get_default_sysroot() -> Path {
    match os::self_exe_path() {
      option::Some(p) => p.pop(),
      option::None => fail ~"can't determine value for sysroot"
    }
}

fn get_sysroot(maybe_sysroot: Option<Path>) -> Path {
    match maybe_sysroot {
      option::Some(sr) => sr,
      option::None => get_default_sysroot()
    }
}

fn get_cargo_sysroot() -> Result<Path, ~str> {
    result::Ok(get_default_sysroot().push_many([libdir(), ~"cargo"]))
}

fn get_cargo_root() -> Result<Path, ~str> {
    match os::getenv(~"CARGO_ROOT") {
        Some(_p) => result::Ok(Path(_p)),
        None => match os::homedir() {
          Some(_q) => result::Ok(_q.push(".cargo")),
          None => result::Err(~"no CARGO_ROOT or home directory")
        }
    }
}

fn get_cargo_root_nearest() -> Result<Path, ~str> {
    do result::chain(get_cargo_root()) |p| {
        let cwd = os::getcwd();
        let cwd_cargo = cwd.push(".cargo");
        let mut par_cargo = cwd.pop().push(".cargo");
        let mut rslt = result::Ok(cwd_cargo);

        if !os::path_is_dir(&cwd_cargo) && cwd_cargo != p {
            while par_cargo != p {
                if os::path_is_dir(&par_cargo) {
                    rslt = result::Ok(par_cargo);
                    break;
                }
                if par_cargo.components.len() == 1 {
                    // We just checked /.cargo, stop now.
                    break;
                }
                par_cargo = par_cargo.pop().pop().push(".cargo");
            }
        }
        rslt
    }
}

fn get_cargo_lib_path() -> Result<Path, ~str> {
    do result::chain(get_cargo_root()) |p| {
        result::Ok(p.push(libdir()))
    }
}

fn get_cargo_lib_path_nearest() -> Result<Path, ~str> {
    do result::chain(get_cargo_root_nearest()) |p| {
        result::Ok(p.push(libdir()))
    }
}

// The name of the directory rustc expects libraries to be located.
// On Unix should be "lib", on windows "bin"
fn libdir() -> ~str {
   let libdir = env!("CFG_LIBDIR");
   if str::is_empty(libdir) {
      fail ~"rustc compiled without CFG_LIBDIR environment variable";
   }
   libdir
}
