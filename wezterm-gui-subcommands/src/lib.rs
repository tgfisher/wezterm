use clap::{Parser, ValueHint};
use config::{Dimension, GeometryOrigin, SshParameters};
use std::ffi::OsString;
use std::str::FromStr;

pub const DEFAULT_WINDOW_CLASS: &str = "org.wezfurlong.wezterm";

/// Helper for parsing config overrides
pub fn name_equals_value(arg: &str) -> Result<(String, String), String> {
    if let Some(eq) = arg.find('=') {
        let (left, right) = arg.split_at(eq);
        let left = left.trim();
        let right = right[1..].trim();
        if left.is_empty() || right.is_empty() {
            return Err(format!(
                "Got empty name/value `{}`; expected name=value",
                arg
            ));
        }
        Ok((left.to_string(), right.to_string()))
    } else {
        Err(format!("Expected name=value, but got {}", arg))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuiPosition {
    pub x: Dimension,
    pub y: Dimension,
    pub origin: GeometryOrigin,
}

impl GuiPosition {
    fn parse_dim(s: &str) -> anyhow::Result<Dimension> {
        if let Some(v) = s.strip_suffix("px") {
            Ok(Dimension::Pixels(v.parse()?))
        } else if let Some(v) = s.strip_suffix("%") {
            Ok(Dimension::Percent(v.parse::<f32>()? / 100.))
        } else {
            Ok(Dimension::Pixels(s.parse()?))
        }
    }

    fn parse_x_y(s: &str) -> anyhow::Result<(Dimension, Dimension)> {
        let fields: Vec<_> = s.split(',').collect();
        if fields.len() != 2 {
            anyhow::bail!("expected x,y coordinates");
        }
        Ok((Self::parse_dim(fields[0])?, Self::parse_dim(fields[1])?))
    }

    fn parse_origin(s: &str) -> GeometryOrigin {
        match s {
            "screen" => GeometryOrigin::ScreenCoordinateSystem,
            "main" => GeometryOrigin::MainScreen,
            "active" => GeometryOrigin::ActiveScreen,
            name => GeometryOrigin::Named(name.to_string()),
        }
    }
}

impl FromStr for GuiPosition {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<GuiPosition> {
        let fields: Vec<_> = s.split(':').collect();
        if fields.len() == 2 {
            let origin = Self::parse_origin(fields[0]);
            let (x, y) = Self::parse_x_y(fields[1])?;
            return Ok(GuiPosition { x, y, origin });
        }
        if fields.len() == 1 {
            let (x, y) = Self::parse_x_y(fields[0])?;
            return Ok(GuiPosition {
                x,
                y,
                origin: GeometryOrigin::ScreenCoordinateSystem,
            });
        }
        anyhow::bail!("invalid position spec {}", s);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn xy() {
        assert_eq!(
            GuiPosition::from_str("10,20").unwrap(),
            GuiPosition {
                x: Dimension::Pixels(10.),
                y: Dimension::Pixels(20.),
                origin: GeometryOrigin::ScreenCoordinateSystem
            }
        );

        assert_eq!(
            GuiPosition::from_str("screen:10,20").unwrap(),
            GuiPosition {
                x: Dimension::Pixels(10.),
                y: Dimension::Pixels(20.),
                origin: GeometryOrigin::ScreenCoordinateSystem
            }
        );
    }

    #[test]
    fn named() {
        assert_eq!(
            GuiPosition::from_str("hdmi-1:10,20").unwrap(),
            GuiPosition {
                x: Dimension::Pixels(10.),
                y: Dimension::Pixels(20.),
                origin: GeometryOrigin::Named("hdmi-1".to_string()),
            }
        );
    }

    #[test]
    fn active() {
        assert_eq!(
            GuiPosition::from_str("active:10,20").unwrap(),
            GuiPosition {
                x: Dimension::Pixels(10.),
                y: Dimension::Pixels(20.),
                origin: GeometryOrigin::ActiveScreen
            }
        );
    }

    #[test]
    fn main() {
        assert_eq!(
            GuiPosition::from_str("main:10,20").unwrap(),
            GuiPosition {
                x: Dimension::Pixels(10.),
                y: Dimension::Pixels(20.),
                origin: GeometryOrigin::MainScreen
            }
        );
    }
}

#[derive(Debug, Parser, Default, Clone)]
#[clap(trailing_var_arg = true)]
pub struct StartCommand {
    /// If true, do not connect to domains marked as connect_automatically
    /// in your wezterm configuration file.
    #[clap(long = "no-auto-connect")]
    pub no_auto_connect: bool,

    /// If enabled, don't try to ask an existing wezterm GUI instance
    /// to start the command.  Instead, always start the GUI in this
    /// invocation of wezterm so that you can wait for the command
    /// to complete by waiting for this wezterm process to finish.
    #[clap(long = "always-new-process")]
    pub always_new_process: bool,

    /// Specify the current working directory for the initially
    /// spawned program
    #[clap(long = "cwd", parse(from_os_str), value_hint=ValueHint::DirPath)]
    pub cwd: Option<OsString>,

    /// Override the default windowing system class.
    /// The default is "org.wezfurlong.wezterm".
    /// Under X11 and Windows this changes the window class.
    /// Under Wayland this changes the app_id.
    /// This changes the class for all windows spawned by this
    /// instance of wezterm, including error, update and ssh
    /// authentication dialogs.
    #[clap(long = "class")]
    pub class: Option<String>,

    /// Override the default workspace with the provided name.
    /// The default is "default".
    #[clap(long = "workspace")]
    pub workspace: Option<String>,

    /// Override the position for the initial window launched by this process.
    ///
    /// --position 10,20          to set x=10, y=20 in screen coordinates
    /// --position screen:10,20   to set x=10, y=20 in screen coordinates
    /// --position main:10,20     to set x=10, y=20 relative to the main monitor
    /// --position active:10,20   to set x=10, y=20 relative to the active monitor
    /// --position HDMI-1:10,20   to set x=10, y=20 relative to the monitor named HDMI-1
    #[clap(long, verbatim_doc_comment)]
    pub position: Option<GuiPosition>,

    /// Instead of executing your shell, run PROG.
    /// For example: `wezterm start -- bash -l` will spawn bash
    /// as if it were a login shell.
    #[clap(parse(from_os_str), value_hint=ValueHint::CommandWithArguments, multiple_values=true)]
    pub prog: Vec<OsString>,
}

#[derive(Debug, Parser, Clone)]
#[clap(trailing_var_arg = true)]
pub struct SshCommand {
    /// Specifies the remote system using the form:
    /// `[username@]host[:port]`.
    /// If `username@` is omitted, then your local $USER is used
    /// instead.
    /// If `:port` is omitted, then the standard ssh port (22) is
    /// used instead.
    pub user_at_host_and_port: SshParameters,

    /// Override specific SSH configuration options.
    /// `wezterm ssh` is able to parse some (but not all!) options
    /// from your `~/.ssh/config` and `/etc/ssh/ssh_config` files.
    /// This command line switch allows you to override or otherwise
    /// specify ssh_config style options.
    ///
    /// For example:
    ///
    /// `wezterm ssh -oIdentityFile=/secret/id_ed25519 some-host`
    #[clap(
        long = "ssh-option",
        short = 'o',
        name = "name=value",
        parse(try_from_str = name_equals_value),
        number_of_values = 1)]
    pub config_override: Vec<(String, String)>,

    /// Enable verbose ssh protocol tracing.
    /// The trace information is printed to the stderr stream of
    /// the process.
    #[clap(short = 'v')]
    pub verbose: bool,

    /// Override the default windowing system class.
    /// The default is "org.wezfurlong.wezterm".
    /// Under X11 and Windows this changes the window class.
    /// Under Wayland this changes the app_id.
    /// This changes the class for all windows spawned by this
    /// instance of wezterm, including error, update and ssh
    /// authentication dialogs.
    #[clap(long = "class")]
    pub class: Option<String>,
    /// Override the position for the initial window launched by this process.
    ///
    /// --position 10,20          to set x=10, y=20 in screen coordinates
    /// --position screen:10,20   to set x=10, y=20 in screen coordinates
    /// --position main:10,20     to set x=10, y=20 relative to the main monitor
    /// --position active:10,20   to set x=10, y=20 relative to the active monitor
    /// --position HDMI-1:10,20   to set x=10, y=20 relative to the monitor named HDMI-1
    #[clap(long, verbatim_doc_comment)]
    pub position: Option<GuiPosition>,

    /// Instead of executing your shell, run PROG.
    /// For example: `wezterm ssh user@host -- bash -l` will spawn bash
    /// as if it were a login shell.
    #[clap(parse(from_os_str), value_hint=ValueHint::CommandWithArguments, multiple_values=true)]
    pub prog: Vec<OsString>,
}

#[derive(Debug, Parser, Clone)]
pub struct SerialCommand {
    /// Set the baud rate.  The default is 9600 baud.
    #[clap(long = "baud")]
    pub baud: Option<usize>,

    /// Override the default windowing system class.
    /// The default is "org.wezfurlong.wezterm".
    /// Under X11 and Windows this changes the window class.
    /// Under Wayland this changes the app_id.
    /// This changes the class for all windows spawned by this
    /// instance of wezterm, including error, update and ssh
    /// authentication dialogs.
    #[clap(long = "class")]
    pub class: Option<String>,
    /// Override the position for the initial window launched by this process.
    ///
    /// --position 10,20          to set x=10, y=20 in screen coordinates
    /// --position screen:10,20   to set x=10, y=20 in screen coordinates
    /// --position main:10,20     to set x=10, y=20 relative to the main monitor
    /// --position active:10,20   to set x=10, y=20 relative to the active monitor
    /// --position HDMI-1:10,20   to set x=10, y=20 relative to the monitor named HDMI-1
    #[clap(long, verbatim_doc_comment)]
    pub position: Option<GuiPosition>,

    /// Specifies the serial device name.
    /// On Windows systems this can be a name like `COM0`.
    /// On posix systems this will be something like `/dev/ttyUSB0`
    #[clap(parse(from_os_str))]
    pub port: OsString,
}

#[derive(Debug, Parser, Clone)]
#[clap(trailing_var_arg = true)]
pub struct ConnectCommand {
    /// Name of the multiplexer domain section from the configuration
    /// to which you'd like to connect
    pub domain_name: String,

    /// Override the default windowing system class.
    /// The default is "org.wezfurlong.wezterm".
    /// Under X11 and Windows this changes the window class.
    /// Under Wayland this changes the app_id.
    /// This changes the class for all windows spawned by this
    /// instance of wezterm, including error, update and ssh
    /// authentication dialogs.
    #[clap(long = "class")]
    pub class: Option<String>,

    /// Override the default workspace with the provided name.
    /// The default is "default".
    #[clap(long = "workspace")]
    pub workspace: Option<String>,
    /// Override the position for the initial window launched by this process.
    ///
    /// --position 10,20          to set x=10, y=20 in screen coordinates
    /// --position screen:10,20   to set x=10, y=20 in screen coordinates
    /// --position main:10,20     to set x=10, y=20 relative to the main monitor
    /// --position active:10,20   to set x=10, y=20 relative to the active monitor
    /// --position HDMI-1:10,20   to set x=10, y=20 relative to the monitor named HDMI-1
    #[clap(long, verbatim_doc_comment)]
    pub position: Option<GuiPosition>,

    /// Instead of executing your shell, run PROG.
    /// For example: `wezterm start -- bash -l` will spawn bash
    /// as if it were a login shell.
    #[clap(parse(from_os_str), value_hint=ValueHint::CommandWithArguments, multiple_values=true)]
    pub prog: Vec<OsString>,
}

#[derive(Debug, Parser, Clone)]
pub struct LsFontsCommand {
    /// Whether to list all fonts available to the system
    #[clap(long = "list-system")]
    pub list_system: bool,

    /// Explain which fonts are used to render the supplied text string
    #[clap(long = "text", conflicts_with = "list-system")]
    pub text: Option<String>,
}

#[derive(Debug, Parser, Clone)]
pub struct ShowKeysCommand {}
