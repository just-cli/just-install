use just_core::kernel::{Kernel, LocalPackage, PackageShims};
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
        use just_download::download;
        use just_extract::extract;
        use just_fetch::Fetch;
        use log::info;

        let mut fetch = Fetch::new(&self.manifest, &mut self.kernel.versions);
        if fetch.needs_fetch() {
            fetch.fetch_all_versions()?;
        }

        let package = &self.manifest.package;
        if self
            .kernel
            .packages
            .is_installed(&package.name, self.req.clone())
        {
            Ok(())
        } else if let Some((version, path)) = self
            .kernel
            .downloads
            .get_download(&package.name, &self.req.clone().unwrap())
        {
            let local = LocalPackage {
                package,
                version,
                path,
            };

            self.add_package(&local);
            create_shims(&self.kernel, &local)
        } else {
            let name = package.name.as_str();

            info!("Downloading package {}...", name);
            let info = download(self.manifest, self.req.clone())?;
            info!("Extracting package {}...", name);
            extract(&info.uncompressed_path, &info.compressed_path).and_then(|_| {
                let local = LocalPackage {
                    package,
                    version: &info.version,
                    path: &info.uncompressed_path,
                };

                self.kernel
                    .downloads
                    .add_download(&local, &self.kernel.path.download_path);
                self.add_package(&local);
                create_shims(&self.kernel, &local)
            })
        }
    }

    fn add_package(&mut self, local: &LocalPackage) {
        self.kernel
            .packages
            .add_package(local.package, local.version);
        self.kernel
            .versions
            .add_version(local.package, local.version);
    }
}

fn create_shims(shims: &PackageShims, local: &LocalPackage) -> BoxedResult<()> {
    shims.create_shims(local)
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
