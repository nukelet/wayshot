use clap::{arg, Command};

pub fn set_flags() -> Command<'static> {
    let app = Command::new("wayshot")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Simple screenshot tool for wlroots based compositors")
        .arg(
            arg!(-d - -debug)
                .required(false)
                .takes_value(false)
                .help("Enable debug mode"),
        )
        .arg(
            arg!(-s --slurp <GEOMETRY>)
                .required(false)
                .takes_value(true)
                .help("Choose a portion of your display to screenshot using slurp"),
        )
        .arg(
            arg!(-f - -file <FILE_PATH>)
                .required(false)
                .conflicts_with("stdout")
                .takes_value(true)
                .help("Mention a custom file path"),
        )
        .arg(
            arg!(-c - -cursor)
                .required(false)
                .takes_value(false)
                .help("Enable cursor in screenshots"),
        )
        .arg(
            arg!(--stdout)
                .required(false)
                .conflicts_with("file")
                .takes_value(false)
                .help("Output the image data to standard out"),
        )
        .arg(
            arg!(-e --extension <FILE_EXTENSION>)
                .required(false)
                .takes_value(true)
                .default_value("png")
                .help("Set image encoder (Png is default)"),
        )
        .arg(
            arg!(-l - -listoutputs)
                .required(false)
                .takes_value(false)
                .help("List all valid outputs"),
        )
        .arg(
            arg!(-o --output <OUTPUT>)
                .required(false)
                .takes_value(true)
                .conflicts_with("slurp")
                .help("Choose a particular display to screenshot"),
        )
        .arg(
            arg!(-C --png_compression_type <TYPE>)
                .required(false)
                .takes_value(true)
                .default_value("fast")
                .help("Set png compression type"),
        )
        .arg(
            arg!(-F --png_filter_type <TYPE>)
                .required(false)
                .takes_value(true)
                .default_value("adaptive")
                .help("Set png compression type"),
        );
    app
}
