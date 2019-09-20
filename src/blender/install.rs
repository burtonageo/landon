use crate::blender;
use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::process::Command;

/// Install the blender mesh exporter addon.
///
/// This gives you access to `bpy.ops.import_export.mesh2json()` from Blender
pub fn install_mesh_to_json() -> std::io::Result<()> {
    // Write our addon to a tmp file. Our `install_mesh_to_json_script` will look for this tmp file
    // when installing the addon.
    let addon_file_path = temp_dir().join("blender-mesh-to-json.py");
    let mesh_to_json_addon = include_str!("../../blender-mesh-to-json.py");
    let mut addon_file = File::create(&addon_file_path)?;
    addon_file.write_all(mesh_to_json_addon.as_bytes())?;

    let install_mesh_to_json_script = format!(r#"
# Install the addon and save the user's preferences
import bpy
import os

# Get the absolute path to the addon
dir = os.path.dirname(__file__)
addonFilePath = '{}'

# Install the addon, enable it and save the user's preferences so that it
# is available whenever Blender is opened in the future
bpy.ops.preferences.addon_install(filepath=addonFilePath)
bpy.ops.preferences.addon_enable(module='blender-mesh-to-json')
bpy.ops.wm.save_userpref()
    "#,
    addon_file_path.display());

    Command::new(blender::exe())
        .arg("--background")
        .args(&["--python-expr", &install_mesh_to_json_script])
        .spawn()?
        .wait()?;

    Ok(())
}

/// Install the blender armature exporter addon.
///
/// This gives you access to `bpy.ops.import_export.armature2json()` from Blender
pub fn install_armature_to_json() -> std::io::Result<()> {
    // Write our addon to a tmp file. Our `install_armature_to_json_script` will look for this tmp file
    // when installing the addon.
    let addon_file_path = temp_dir().join("blender-armature-to-json.py");
    let armature_to_json = include_str!("../../blender-armature-to-json.py");
    let mut addon_file = File::create(&addon_file_path)?;
    addon_file.write_all(armature_to_json.as_bytes())?;

    let install_armature_to_json_script = format!(r#"
import bpy

addonFilePath = '{}'

# Install the addon, enable it and save the user's preferences so that it
# is available whenever Blender is opened in the future
bpy.ops.preferences.addon_install(filepath=addonFilePath)
bpy.ops.preferences.addon_enable(module='blender-armature-to-json')
bpy.ops.wm.save_userpref()
    "#,
    addon_file_path.display());

    Command::new(blender::exe())
        .arg("--background")
        .args(&["--python-expr", &install_armature_to_json_script])
        .spawn()?
        .wait()?;

    Ok(())
}
