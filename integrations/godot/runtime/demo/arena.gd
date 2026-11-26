# SPDX-License-Identifier: MPL-2.0
extends "res://match/battle.gd"

signal finished(caption: String)
signal pause_requested()

var mode := "duel"
var roster := PackedStringArray(["amber", "indigo"])
var character_dirs: Dictionary = {}
var suspended := false
var boxes_visible := false
var infinite_health := true
var slow_motion := false
var frozen := false
var recording := false
var replaying := false
var dummy_mode := 0
var _guard_after_hit := false
var _slow_tick := 0
var _step_pending := false
var replay_path := "user://training.ofr"
var replay_message: String:
	get: return $HUD.message.text
var _recorded_frames := 0
const ReplayFile = preload("res://demo/replay_file.gd")
var _file_thread: Thread
var _loading_file := false
var _ended := false
var _pad_ids := PackedInt32Array()

const PAD := [JOY_BUTTON_DPAD_LEFT, JOY_BUTTON_DPAD_RIGHT, JOY_BUTTON_DPAD_UP,
	JOY_BUTTON_DPAD_DOWN, JOY_BUTTON_A, JOY_BUTTON_B, JOY_BUTTON_LEFT_SHOULDER,
	JOY_BUTTON_X, JOY_BUTTON_Y]
const TAPE_LIMIT := 36000 # Ten minutes at simulation speed.

func _ready() -> void:
	if sprites.is_empty():
		_setup_arena()
	_dirs = _bundles.open_dirs(roster, character_dirs)
	_pad_ids = Input.get_connected_joypads()
	if not Input.joy_connection_changed.is_connected(_controllers_changed):
		Input.joy_connection_changed.connect(_controllers_changed)
	restart()

func _setup_arena() -> void:
	_actions.clear()
	for i in range(roster.size()):
		_actions.append([])
		_bind_fighter(i, ACTIONS.size())
		_add_fighter_nodes()
	audio = $Audio
	audio.bus = &"SFX"
	_sounds["Hit"] = _tone(180, 47, 0.115)
	_sounds["Blocked"] = _tone(470, 140, 0.085)
	$HUD.configure(mode, roster.size())
	health_bars = $HUD.bars
	$HUD.training_action.connect(_training_action)

func _controllers_changed(_device: int, _connected: bool) -> void:
	_pad_ids = Input.get_connected_joypads()

func restart() -> bool:
	_finish_file(true)
	recording = false
	replaying = false
	return _new_round()

func _new_round() -> bool:
	_ended = false
	_step_pending = false
	_guard_after_hit = false
	_slow_tick = 0
	if mode == "training" and match_instance != null:
		if match_instance.training_reset() != 0:
			return _fail("Could not reset the training match.")
	else:
		_close()
		if _dirs.size() != roster.size() or not ClassDB.class_exists(&"OFEMatch"):
			return _fail("Could not load characters or native extension. Esc to leave.")
		match_instance = ClassDB.instantiate(&"OFEMatch")
		if not match_instance.has_method("open_roster") or match_instance.open_roster(_dirs, mode) != 0:
			match_instance = null
			return _fail("Could not open this match. Esc to leave.")
	if mode == "training" and match_instance.set_infinite_health(infinite_health) != 0:
		return _fail("Could not configure training health. Esc to leave.")
	$HUD.message.text = ""
	set_physics_process(true)
	return _present()

func set_suspended(value: bool) -> void:
	suspended = value
	_step_pending = false
	if audio:
		audio.stream_paused = value

func _unhandled_input(event: InputEvent) -> void:
	if suspended or _ended:
		return
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		pause_requested.emit()
		return
	if mode == "training" and event is InputEventKey and event.pressed and not event.echo:
		var code: int = event.physical_keycode if event.physical_keycode else event.keycode
		if code >= KEY_F1 and code <= KEY_F11:
			get_viewport().set_input_as_handled()
			_training_action(code - KEY_F1)

func _training_action(index: int) -> void:
	if suspended or match_instance == null or replay_io_pending():
		return
	match index:
		0:
			if not replaying:
				dummy_mode = (dummy_mode + 1) % 3
				_guard_after_hit = false
		1:
			boxes_visible = not boxes_visible
			$Projectiles.queue_redraw()
		2:
			if not recording and not replaying and match_instance.set_infinite_health(not infinite_health) == 0:
				infinite_health = not infinite_health
		3:
			restart()
		4:
			slow_motion = not slow_motion
			_slow_tick = 0
		5:
			frozen = not frozen
			_step_pending = false
		6:
			frozen = true
			_step_pending = true
		7:
			if recording:
				match_instance.record_end()
				recording = false
			elif restart() and match_instance.record_begin() == 0:
				_recorded_frames = 0
				recording = true
				frozen = false
		8:
			if _recorded_frames > 0:
				_start_replay(match_instance.play_begin())
		9:
			_begin_file(false)
		10:
			_begin_file(true)
	_update_training()

func _start_replay(status: int) -> void:
	var info: Dictionary = match_instance.replay_info()
	if status != 0:
		$HUD.message.text = info.get("error", "Could not open replay.")
		return
	$HUD.message.text = ""
	_recorded_frames = int(info.frames)
	infinite_health = info.infinite_health
	recording = false
	replaying = true
	frozen = false
	_step_pending = false
	_guard_after_hit = false
	_slow_tick = 0
	_present()

func replay_io_pending() -> bool:
	return _file_thread != null

func _begin_file(loading: bool) -> void:
	if not loading and (recording or replaying or _recorded_frames == 0):
		$HUD.message.text = "Finish a recording before saving."
		return
	_loading_file = loading
	var operation := ReplayFile.load_recording if loading else ReplayFile.save
	var worker := Thread.new()
	if worker.start(operation.bind(match_instance, ProjectSettings.globalize_path(replay_path))) != OK:
		$HUD.message.text = "Could not start replay file operation."
		return
	_file_thread = worker
	$HUD.message.text = "Checking replay…" if loading else "Saving replay…"

func _finish_file(wait := false) -> void:
	if _file_thread == null or (not wait and _file_thread.is_alive()):
		return
	var message: String = _file_thread.wait_to_finish()
	_file_thread = null
	if not message.is_empty():
		$HUD.message.text = message
	elif _loading_file:
		_start_replay(0)
	else:
		$HUD.message.text = "Replay saved  ·  %s" % replay_path.get_file()

func _exit_tree() -> void:
	if _file_thread != null:
		_file_thread.wait_to_finish()
		_file_thread = null
	super()

func _input_mask(player: int) -> int:
	var mask := 0
	var pad := _pad_ids[player] if player < _pad_ids.size() else -1
	var moves := int(current.fighters[player].get("move_count", 1))
	for bit in range(ACTIONS.size()):
		var down := Input.is_action_pressed(_actions[player][bit])
		if pad >= 0:
			down = down or Input.is_joy_button_pressed(pad, PAD[bit])
		if down:
			if bit < 7:
				mask |= 1 << bit
			elif bit - 5 <= moves:
				mask = (mask & 0x6f) | ((bit - 5) << 8)
	if mask & 3 == 3:
		mask &= ~3
	return mask

func _physics_process(_delta: float) -> void:
	if replay_io_pending():
		_finish_file()
		return
	if suspended or _ended or match_instance == null:
		return
	if frozen and not _step_pending:
		return
	if slow_motion and not _step_pending:
		_slow_tick = (_slow_tick + 1) % 4
		if _slow_tick != 0:
			return
	_step_pending = false
	var frame := int(current.frame) + 1
	if replaying:
		if match_instance.play_step() != 0:
			replaying = false
			frozen = true
			$HUD.message.text = match_instance.replay_info().get("error", "Replay diverged.")
			_update_training()
			return
		if frame == _recorded_frames:
			replaying = false
			frozen = true
	else:
		var masks := PackedInt32Array()
		for i in range(roster.size()):
			if mode == "training" and i == 1:
				masks.append(32 if dummy_mode == 1 or _guard_after_hit else 0)
			else:
				masks.append(_input_mask(i))
		if match_instance.step_roster(frame, masks) != 0:
			_fail("Frame %d failed. Esc to leave." % frame)
			return
		if recording:
			_recorded_frames += 1
			if _recorded_frames >= TAPE_LIMIT:
				match_instance.record_end()
				recording = false
	if _present() and mode != "training":
		_check_winner()

func _present() -> bool:
	current = match_instance.snapshot()
	if current.get("status", -1) != 0 or current.get("fighters", []).size() != roster.size():
		return _fail("Could not read the match. Esc to leave.")
	if not _render_snapshot():
		return false
	for i in range(sprites.size()):
		var fighter: Dictionary = current.fighters[i]
		sprites[i].modulate.a = 0.3 if fighter.hp <= 0 else 1.0
		$HUD.track(i, sprites[i].position, fighter.hp > 0)
	if mode == "training":
		if dummy_mode == 2:
			for event in current.events:
				if event.kind == "Hit" and int(event.target) == 2:
					_guard_after_hit = true
				elif event.kind == "Blocked" and int(event.target) == 2:
					_guard_after_hit = false
		_update_training()
	return true

func _check_winner() -> void:
	var alive := {}
	for fighter in current.fighters:
		if fighter.hp > 0:
			alive[int(fighter.team)] = fighter
	if alive.size() > 1:
		return
	_ended = true
	if alive.is_empty():
		finished.emit("Draw")
	elif mode == "team":
		finished.emit("Team %d wins" % int(alive.keys()[0]))
	else:
		var winner: Dictionary = alive.values()[0]
		finished.emit("P%d  ·  %s wins" % [winner.id, winner.name])

func _update_training() -> void:
	$HUD.training_status(current, dummy_mode, boxes_visible, infinite_health, slow_motion,
		frozen, recording, replaying, _recorded_frames)

func _on_projectiles_draw() -> void:
	super()
	if boxes_visible:
		for fighter in current.get("fighters", []):
			for box in fighter.boxes:
				var a := _project(box.x, box.z, box.h + box.height)
				var b := _project(box.x + box.width, box.z, box.h)
				var rect := Rect2(a, b - a)
				var tint: Color = {"hit": Color.CORAL, "hurt": Color.AQUAMARINE, "push": Color.GOLD}[box.kind]
				$Projectiles.draw_rect(rect, Color(tint, 0.15))
				$Projectiles.draw_rect(rect, tint, false, 2)
