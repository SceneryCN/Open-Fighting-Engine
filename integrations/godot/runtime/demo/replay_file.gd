# SPDX-License-Identifier: MPL-2.0
extends RefCounted

# Runs on a worker. No scene nodes are accessed while the native match is borrowed.
static func save(match_handle: RefCounted, path: String) -> String:
	var bytes: PackedByteArray = match_handle.replay_export()
	if bytes.is_empty():
		return "Could not encode the recording."
	var temporary := "%s.%d.%d.tmp" % [path, OS.get_process_id(), match_handle.get_instance_id()]
	var file := FileAccess.open(temporary, FileAccess.WRITE)
	if file == null:
		return "Could not create the replay file."
	var stored := file.store_buffer(bytes)
	file.flush()
	stored = stored and file.get_error() == OK
	file.close()
	if not stored or DirAccess.rename_absolute(temporary, path) != OK:
		DirAccess.remove_absolute(temporary)
		return "Could not save the recording."
	return ""

static func load_recording(match_handle: RefCounted, path: String) -> String:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null or file.get_length() <= 0 or file.get_length() > 64 * 1024 * 1024:
		return "No readable replay at %s" % path.get_file()
	var bytes := file.get_buffer(file.get_length())
	file.close()
	if match_handle.play_load(bytes) != 0:
		return match_handle.replay_info().get("error", "Could not load the replay.")
	return ""
