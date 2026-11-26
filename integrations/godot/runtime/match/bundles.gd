# SPDX-License-Identifier: MPL-2.0
extends RefCounted

# Rust reads local files. Unpack the trusted, shipped OFC archives into a private
# match directory. User-supplied directories are opened in place and never removed.
var directory := ""

func open_dirs() -> PackedStringArray:
	var result := PackedStringArray()
	var options := OS.get_cmdline_user_args()
	for slot in [1, 2]:
		var prefix := "--p%d-dir=" % slot
		var override := ""
		for arg in options:
			if arg.begins_with(prefix):
				override = arg.trim_prefix(prefix)
				if override.is_empty():
					return PackedStringArray()
		if not override.is_empty():
			result.append(ProjectSettings.globalize_path(override))
			continue
		if directory.is_empty():
			directory = OS.get_temp_dir().path_join("ofe-%d-%d-%d" % [OS.get_process_id(), Time.get_ticks_usec(), get_instance_id()])
			if DirAccess.make_dir_absolute(directory) != OK:
				directory = ""
				return PackedStringArray()
		var actor := "amber" if slot == 1 else "indigo"
		var folder := directory.path_join(actor)
		var zip := ZIPReader.new()
		if zip.open("res://match/%s.ofc" % actor) != OK:
			return PackedStringArray()
		for entry in zip.get_files():
			if entry.is_absolute_path() or "\\" in entry or ":" in entry or ".." in entry.split("/"):
				return PackedStringArray()
			var path := folder.path_join(entry)
			if DirAccess.make_dir_recursive_absolute(path.get_base_dir()) != OK:
				return PackedStringArray()
			var bytes := zip.read_file(entry)
			var out := FileAccess.open(path, FileAccess.WRITE)
			if not out or not out.store_buffer(bytes):
				return PackedStringArray()
			out.close()
		zip.close()
		result.append(folder)
	return result

func clear() -> void:
	if not directory.is_empty():
		_remove(directory)
		directory = ""

func _remove(path: String) -> void:
	var dir := DirAccess.open(path)
	if dir == null:
		return
	for item in dir.get_files():
		dir.remove(item)
	for item in dir.get_directories():
		_remove(path.path_join(item))
	DirAccess.remove_absolute(path)
