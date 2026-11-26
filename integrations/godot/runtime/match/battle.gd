# SPDX-License-Identifier: MPL-2.0
extends Node2D

@export var depth_projection := 0.054
var match_instance: RefCounted
var current: Dictionary = {}
var textures: Dictionary = {}
var sprites: Array[Sprite2D] = []
var health_bars: Array[ProgressBar] = []
var shadows: Array[Polygon2D] = []
var audio: AudioStreamPlayer
var sound_count := 0
var _sounds: Dictionary = {}
var _bundles := preload("res://match/bundles.gd").new()
var _dirs := PackedStringArray()
var _camera_x := 0.0
var _unit := 35.0
var _actions := [[], []]
var _qa: RefCounted

func _ready() -> void:
	if "--p0-test" in OS.get_cmdline_user_args():
		get_tree().change_scene_to_file.call_deferred("res://proof.tscn")
		return
	if sprites.is_empty():
		_setup()
	_dirs = _bundles.open_dirs()
	if not restart():
		return
	if "--p4-test" in OS.get_cmdline_user_args() and not get_tree().root.has_meta(&"p4_qa_started"):
		get_tree().root.set_meta(&"p4_qa_started", true)
		_qa = preload("res://match/smoke.gd").new()
		_qa.run.call_deferred(self)

func _setup() -> void:
	var keys := [[KEY_A, KEY_D, KEY_W, KEY_S, KEY_F, KEY_G, KEY_H],
		[KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN, KEY_J, KEY_K, KEY_L]]
	var suffixes := ["left", "right", "jump", "lane", "attack", "guard", "dash"]
	for i in range(2):
		for n in range(suffixes.size()):
			var action := StringName("p%d_%s" % [i + 1, suffixes[n]])
			_bind(action, keys[i][n])
			_actions[i].append(action)
		_add_fighter_nodes()
	_bind(&"match_restart", KEY_R)
	health_bars = $HUD.bars
	audio = $Audio
	_sounds["Hit"] = _tone(180.0, 47.0, 0.115)
	_sounds["Blocked"] = _tone(470.0, 140.0, 0.085)

func _add_fighter_nodes() -> void:
	var shadow := Polygon2D.new()
	var points := PackedVector2Array()
	for n in range(24):
		var angle := n * TAU / 24.0
		points.append(Vector2(cos(angle) * 40, sin(angle) * 6))
	shadow.polygon = points
	shadow.color = Color("#243a40bc")
	$Actors.add_child(shadow)
	shadows.append(shadow)
	var sprite := Sprite2D.new()
	sprite.centered = false
	sprite.region_enabled = true
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	$Actors.add_child(sprite)
	sprites.append(sprite)

func _bind(action: StringName, code: Key) -> void:
	if InputMap.has_action(action):
		return
	InputMap.add_action(action)
	var event := InputEventKey.new()
	event.physical_keycode = code
	InputMap.action_add_event(action, event)

func _tone(start: float, finish: float, duration: float) -> AudioStreamWAV:
	var wav := AudioStreamWAV.new()
	wav.format = AudioStreamWAV.FORMAT_16_BITS
	wav.mix_rate = 22050
	var count := roundi(duration * wav.mix_rate)
	var samples := PackedByteArray()
	samples.resize(count * 2)
	var phase := 0.0
	for i in range(count):
		var t := float(i) / count
		phase += TAU * lerpf(start, finish, t) / wav.mix_rate
		var envelope := minf(1.0, t * 48.0) * pow(1.0 - t, 2.0)
		samples.encode_s16(i * 2, roundi(sin(phase) * envelope * 22500))
	wav.data = samples
	return wav

func restart() -> bool:
	_close()
	if _dirs.size() != 2 or not ClassDB.class_exists(&"OFEMatch"):
		return _fail("Could not load the native extension or character packages.")
	match_instance = ClassDB.instantiate(&"OFEMatch")
	if match_instance.open(_dirs[0], _dirs[1]) != 0:
		match_instance = null
		return _fail("Could not open characters: %s / %s" % [_dirs[0], _dirs[1]])
	$HUD.message.text = ""
	set_physics_process(true)
	return _present()

func _physics_process(_delta: float) -> void:
	if match_instance == null:
		return
	if Input.is_action_just_pressed(&"match_restart"):
		restart()
		return
	var frame := int(current.frame) + 1
	var status: int = match_instance.step(frame, _input_mask(0), _input_mask(1))
	if status != 0:
		_fail("Frame %d failed (%d)" % [frame, status])
		return
	_present()

func _input_mask(player: int) -> int:
	var mask := 0
	for bit in range(_actions[player].size()):
		if Input.is_action_pressed(_actions[player][bit]):
			mask |= 1 << bit
	if mask & 3 == 3:
		mask &= ~3
	return mask

func _project(x: float, z: float, height: float) -> Vector2:
	return Vector2(480.0 + (x - _camera_x) * _unit, 422.0 - z * depth_projection - height * 8.0)

func _present() -> bool:
	current = match_instance.snapshot()
	if current.get("status", -1) != 0 or current.fighters.size() != 2:
		return _fail("Could not read the match snapshot.")
	return _render_snapshot()

func _render_snapshot() -> bool:
	var left := float(current.fighters[0].x)
	var right := left
	for fighter in current.fighters:
		left = minf(left, fighter.x)
		right = maxf(right, fighter.x)
	_camera_x = (left + right) * 0.5
	_unit = minf(25.0, 756.0 / maxf(1.0, right - left))
	for i in range(current.fighters.size()):
		var f: Dictionary = current.fighters[i]
		var anim: Dictionary = f.animation
		var id := int(anim.image_id)
		if not textures.has(id):
			var img := Image.new()
			if img.load_png_from_buffer(match_instance.image(id)) != OK:
				return _fail("Could not decode image %d" % id)
			textures[id] = ImageTexture.create_from_image(img)
		var sprite := sprites[i]
		sprite.texture = textures[id]
		sprite.region_rect = Rect2(anim.rect[0], anim.rect[1], anim.rect[2], anim.rect[3])
		sprite.offset = -Vector2(anim.origin[0], anim.origin[1])
		var size_factor := 0.66 * (1.0 - 0.00010 * float(f.z))
		sprite.scale = Vector2(size_factor * f.facing, size_factor)
		sprite.position = _project(f.x, f.z, f.h)
		sprite.z_index = 3 if f.z < 500 else 2
		shadows[i].position = _project(f.x, f.z, 0)
		shadows[i].scale = Vector2.ONE * (1.0 - 0.00010 * float(f.z))
	$HUD.present(current)
	for event in current.events:
		if _sounds.has(event.kind):
			audio.stream = _sounds[event.kind]
			audio.play()
			sound_count += 1
	$Projectiles.queue_redraw()
	return true

func _on_projectiles_draw() -> void:
	for p in current.get("projectiles", []):
		var at := _project(p.x, p.z, p.h)
		$Projectiles.draw_circle(at, 7, Color("#ffbd56"))
		$Projectiles.draw_circle(at, 3, Color("#fff5d0"))

func _fail(message: String) -> bool:
	push_error(message)
	$HUD.message.text = message
	set_physics_process(false)
	if "--p4-test" in OS.get_cmdline_user_args():
		get_tree().quit(1)
	return false

func _close() -> void:
	if match_instance != null:
		match_instance.close()
		match_instance = null
	for sprite in sprites:
		sprite.texture = null
	if audio != null:
		audio.stop()
	textures.clear()
	current.clear()

func _exit_tree() -> void:
	_close()
	_bundles.clear()
	_dirs.clear()
	request_ready()
