use interoptopus::util::NamespaceMappings;
use interoptopus::{Error, Interop};
use libdurl::inventory;

#[test]
fn bindings_csharp() -> Result<(), Error> {
    use interoptopus_backend_csharp::overloads::DotNet;
    use interoptopus_backend_csharp::{Config, Generator};

    let config = Config {
        dll_name: "libdurl".to_string(),
        class:"DUrlInterop".to_string(),
        namespace_mappings: NamespaceMappings::new("durl"),
        ..Config::default()
    };

    Generator::new(config, inventory())
        .add_overload_writer(DotNet::new())
        //.add_overload_writer(Unity::new())
        .write_file("DUrlInterop.cs")?;

    Ok(())
}

#[test]
fn bindings_c() -> Result<(), Error> {
    use interoptopus_backend_c::{Config, Generator};

    Generator::new(
        Config {
            ifndef: "check_durl".to_string(),
            ..Config::default()
        },
        inventory(),
    )
        .write_file("durl.h")?;

    Ok(())
}

#[test]
fn bindings_cpython_cffi() -> Result<(), Error> {
    use interoptopus_backend_cpython::{Config, Generator};

    let library = inventory();
    Generator::new(Config::default(), library).write_file("durl.py")?;

    Ok(())
}