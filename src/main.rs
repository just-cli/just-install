use just_core::blueprint::Blueprint;
use just_core::kernel::LocalPackage;
use just_core::result::BoxedResult;
use semver::VersionReq;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "fetch")]
struct Opt {
    #[structopt(long = "package")]
    pub package: Option<String>,
    #[structopt(long = "version")]
    pub version: Option<VersionReq>,
}

fn install(blueprint: &Blueprint, req: Option<VersionReq>) -> BoxedResult<()> {
    use just_core::kernel::Kernel;
    use just_download::download;
    use just_extract::extract;
    use just_fetch::Fetch;
    use log::info;

    let mut kernel = Kernel::load();
    let mut fetch = Fetch::new(&blueprint, &mut kernel.versions);
    if fetch.needs_fetch() {
        fetch.fetch_all_versions()?;
    }

    let package = &blueprint.package;
    if kernel.packages.is_installed(package, req.clone()) {
        Ok(())
    } else if let Some((version, path)) = kernel
        .downloads
        .get_download(&package.name, &req.clone().unwrap())
    {
        let local = LocalPackage {
            package,
            version,
            path,
        };

        kernel.packages.add_package(local.package, local.version);
        kernel.versions.add_version(local.package, local.version);
        kernel.create_shims(&local)
    } else {
        let name = package.name.as_str();

        info!("Downloading package {}...", name);
        let info = download(blueprint, req)?;
        info!("Extracting package {}...", name);
        extract(&info.uncompressed_path, &info.compressed_path).and_then(|_| {
            let local = LocalPackage {
                package,
                version: &info.version,
                path: &info.uncompressed_path,
            };

            kernel
                .downloads
                .add_download(&local, &kernel.path.downloads);
            kernel.packages.add_package(local.package, local.version);
            kernel.versions.add_version(local.package, local.version);
            kernel.create_shims(&local)
        })
    }
}

fn main() {
    use just_core::blueprint::Blueprints;

    let opt: Opt = Opt::from_args();
    if let Some(pkg) = opt.package {
        if let Some(blueprint) = Blueprints::scan().load(&pkg) {
            let req = opt.version.or_else(|| Some(VersionReq::any()));
            install(&blueprint, req)
                .unwrap_or_else(|e| panic!("Could not install package {}: {:?}", pkg, e));
        } else {
            println!("Package {:?} does not exists", pkg);
        }
    }
}
