use just_core::kernel::{Kernel, LocalPackage};
use just_core::manifest::Manifest;
use just_core::result::BoxedResult;
use semver::VersionReq;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "install")]
struct Opt {
    #[structopt(long = "package")]
    pub package: Option<String>,
    #[structopt(long = "version")]
    pub version: Option<VersionReq>,
}

struct Install<'a> {
    kernel: &'a mut Kernel,
    manifest: &'a Manifest,
    req: Option<VersionReq>,
}

impl<'a> Install<'a> {
    fn new(kernel: &'a mut Kernel, manifest: &'a Manifest, req: Option<VersionReq>) -> Self {
        Self {
            kernel,
            manifest,
            req,
        }
    }

    fn install(&mut self) -> BoxedResult<()> {
        use just_core::kernel::PackageShims;
        use just_download::download;
        use just_extract::extract;
        use just_fetch::Fetch;
        use log::info;

        let mut fetch = Fetch::new(&self.manifest, &mut self.kernel.versions);
        if fetch.needs_fetch() {
            fetch.fetch_all_versions()?;
        }

        let package = &self.manifest.package;
        let pkg_name = &package.name;

        if self
            .kernel
            .packages
            .is_installed(pkg_name, self.req.clone())
        {
            let version = self
                .kernel
                .packages
                .get_package_version(pkg_name)
                .expect("No version found although package is installed?!");

            info!("Package {}-{} is already installed", pkg_name, version);

            Ok(())
        } else if let Some((version, path)) = self
            .kernel
            .downloads
            .get_download(pkg_name, &self.req.clone().unwrap())
        {
            info!("Use cached version for installation...");

            let local = LocalPackage {
                package,
                version,
                path,
            };

            self.kernel
                .packages
                .add_package(local.package, local.version);
            self.kernel
                .versions
                .add_version(local.package, local.version);
            self.kernel.create_shims(&local).and_then(|_| {
                info!(
                    "Package {}-{} was successfully installed",
                    pkg_name, version
                );

                Ok(())
            })
        } else {
            info!("Downloading package {}...", pkg_name);
            let info = download(self.manifest, self.req.clone())?;
            info!("Extracting package {}...", pkg_name);
            extract(&info.uncompressed_path, &info.compressed_path).and_then(|_| {
                let local = LocalPackage {
                    package,
                    version: &info.version,
                    path: &info.uncompressed_path,
                };

                self.kernel
                    .downloads
                    .add_download(&local, &self.kernel.path.download_path);
                self.kernel
                    .packages
                    .add_package(local.package, local.version);
                self.kernel
                    .versions
                    .add_version(local.package, local.version);
                self.kernel.create_shims(&local).and_then(|_| {
                    info!(
                        "Package {}-{} was successfully installed",
                        pkg_name, info.version
                    );

                    Ok(())
                })
            })
        }
    }
}

fn main() {
    use just_core::manifest::ManifestFiles;

    let opt: Opt = Opt::from_args();
    if let Some(pkg_name) = opt.package {
        let mut kernel = Kernel::load();

        if let Some(manifest) = ManifestFiles::scan(&kernel).load_manifest(&pkg_name) {
            let mut install = Install::new(
                &mut kernel,
                &manifest,
                opt.version.or_else(|| Some(VersionReq::any())),
            );
            install
                .install()
                .unwrap_or_else(|e| panic!("Could not install package {}: {:?}", pkg_name, e));
        } else {
            println!("Package {:?} does not exists", pkg_name);
        }
    }
}
