# SPDX-License-Identifier: MPL-2.0
extends "res://demo/smoke.gd"

# The harness supplies a private path and launches a fresh process for playback.
func run(root) -> void:
	game = root
	tree = game.get_tree()
	var path := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--qa-replay="):
			path = arg.trim_prefix("--qa-replay=")
	if not path.is_absolute_path():
		error = "expected an isolated absolute replay path"
	elif await _enter("Training", "training", [1, 2]):
		game.arena.replay_path = path
		if "--qa-play" in OS.get_cmdline_user_args():
			await _read_disk()
		else:
			await _write_disk(path)
	await game.stop_audio()
	if error.is_empty():
		print("OFE_P6_PASS")
	else:
		push_error("P6 acceptance: " + error)
	tree.quit(0 if error.is_empty() else 1)

func _write_disk(path: String) -> void:
	var arena = game.arena
	await _key(KEY_F8)
	if not check(arena.recording, "record key did not begin capture"):
		return
	Input.action_press(&"p1_attack")
	for tick in range(5):
		await tree.physics_frame
	Input.action_release(&"p1_attack")
	for tick in range(12):
		await tree.physics_frame
	Input.action_press(&"p1_right")
	for tick in range(8):
		await tree.physics_frame
	Input.action_release(&"p1_right")
	await _key(KEY_F8)
	await _disk_key(KEY_F10)
	if not check(FileAccess.file_exists(path), "F10 did not write the recording: " + arena.replay_message):
		return
	var disk: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not check(disk is Dictionary and disk.get("format") == "OFE_REPLAY" and disk.frames.size() >= 20, "invalid disk recording"):
		return
	print("OFE_P6_RECORDED ", disk.frames.size())

func _read_disk() -> void:
	var arena = game.arena
	var disk: Variant = JSON.parse_string(FileAccess.get_file_as_string(arena.replay_path))
	var length: int = disk.frames.size()
	await _disk_key(KEY_F11)
	if not check(arena.replaying, "F11 in new process failed: " + arena.replay_message):
		return
	for tick in range(length + 5):
		await tree.physics_frame
	if not check(arena.frozen and not arena.replaying and int(arena.current.frame) == length,
			"playback did not freeze at recorded final frame: " + arena.replay_message):
		return
	var fighter: Dictionary = arena.current.fighters[0]
	if not check(fighter.damage > 0 and fighter.x > -2, "replayed hit and walk missing: damage=%s x=%s" % [fighter.damage, fighter.x]):
		return
	var before: Dictionary = arena.current.duplicate(true)
	var original := FileAccess.get_file_as_string(arena.replay_path)
	var altered := original.replace(str(disk.frames[6].hash), "0000000000000000")
	var stream := FileAccess.open(arena.replay_path, FileAccess.WRITE)
	if not check(stream != null and stream.store_string(altered), "could not write QA divergence"):
		return
	stream.close()
	await _disk_key(KEY_F11)
	if not check(arena.frozen and arena.current == before and "frame 7" in arena.replay_message, "corrupt import changed current state or lost diagnostic: " + arena.replay_message):
		return
	await _key(KEY_F9)
	for tick in range(length + 5):
		await tree.physics_frame
	check(arena.frozen and arena.current == before, "failed import replaced previous valid tape")
	stream = FileAccess.open(arena.replay_path, FileAccess.WRITE)
	if stream:
		stream.store_string(original)
		stream.close()

func _disk_key(key: Key) -> void:
	await _key(key)
	while game.arena.replay_io_pending():
		await tree.process_frame
