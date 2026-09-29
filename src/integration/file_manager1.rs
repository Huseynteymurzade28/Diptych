use gio::prelude::*;
use std::path::PathBuf;

// ═══════════════════════════════════════════════
//  org.freedesktop.FileManager1
// ═══════════════════════════════════════════════
//
// The D-Bus service browsers, download managers and IDEs call for "Show in
// folder". Diptych only claims the name when it's the default file
// manager (the `inode/directory` handler) or was started by D-Bus for it,
// so running it next to Dolphin or Nautilus never steals their requests.

pub const NAME: &str = "org.freedesktop.FileManager1";
const PATH: &str = "/org/freedesktop/FileManager1";
pub const DESKTOP_ID: &str = "com.flear.diptych.desktop";

const XML: &str = r#"
<node>
  <interface name="org.freedesktop.FileManager1">
    <method name="ShowFolders">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
    <method name="ShowItems">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
    <method name="ShowItemProperties">
      <arg type="as" name="URIs" direction="in"/>
      <arg type="s" name="StartupId" direction="in"/>
    </method>
  </interface>
</node>"#;

/// What a caller asked for.
pub struct Request {
    pub paths: Vec<PathBuf>,
    /// `true` for ShowItems / ShowItemProperties: select the items in
    /// their folder. `false` for ShowFolders: open the folders.
    pub reveal: bool,
    pub startup_id: String,
}

/// Whether Diptych should claim the name in this process.
pub fn should_own(app: &impl IsA<gio::Application>) -> bool {
    let is_service = app
        .as_ref()
        .flags()
        .contains(gio::ApplicationFlags::IS_SERVICE);
    let is_default = gio::AppInfo::default_for_type("inode/directory", false)
        .and_then(|info| info.id())
        .is_some_and(|id| id == DESKTOP_ID);
    is_service || is_default
}

/// Claims the name and calls `handle` for every request. Another file
/// manager that asks for the name later gets it.
pub fn own(handle: impl Fn(Request) + 'static) -> gio::OwnerId {
    let handle = std::rc::Rc::new(handle);
    gio::bus_own_name(
        gio::BusType::Session,
        NAME,
        gio::BusNameOwnerFlags::ALLOW_REPLACEMENT | gio::BusNameOwnerFlags::DO_NOT_QUEUE,
        move |connection, _| {
            let info = gio::DBusNodeInfo::for_xml(XML)
                .ok()
                .and_then(|node| node.lookup_interface(NAME));
            let Some(info) = info else {
                eprintln!("[dbus] Invalid {} interface description", NAME);
                return;
            };
            let handle = handle.clone();
            let registered = connection
                .register_object(PATH, &info)
                .method_call(move |_, _, _, _, method, params, invocation| {
                    let reveal = match method {
                        "ShowFolders" => false,
                        "ShowItems" | "ShowItemProperties" => true,
                        _ => {
                            invocation.return_error(
                                gio::DBusError::UnknownMethod,
                                &format!("Unknown method {}", method),
                            );
                            return;
                        }
                    };
                    let Some((uris, startup_id)) = params.get::<(Vec<String>, String)>() else {
                        invocation.return_error(gio::DBusError::InvalidArgs, "Expected (as, s)");
                        return;
                    };
                    let paths = uris
                        .iter()
                        .filter_map(|uri| gio::File::for_uri(uri).path())
                        .collect();
                    invocation.return_value(None);
                    handle(Request {
                        paths,
                        reveal,
                        startup_id,
                    });
                })
                .build();
            if let Err(e) = registered {
                eprintln!("[dbus] Couldn't export {}: {}", PATH, e);
            }
        },
        |_, name| println!("[dbus] Serving {}", name),
        |_, name| println!("[dbus] {} is served by another file manager", name),
    )
}
