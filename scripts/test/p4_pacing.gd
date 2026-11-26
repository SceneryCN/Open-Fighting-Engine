# SPDX-License-Identifier: MPL-2.0
extends SceneTree

const WARMUP := 120
const SAMPLES := 600
var match_scene
var frames := 0
var previous := 0
var ticks := PackedInt64Array()
var physics_count := 0
var combat_hits := 0
var start_ticks := 0
var start_hits := 0
var combat_blocks := 0
var start_blocks := 0
var steady := false
var screenshot_path := ""
var restart_costs := PackedInt64Array()
var frame_cpu_us := PackedInt64Array()
var tick_start := 0
var last_simulation_frame := -1


class AfterMatch extends Node:
	var ended: Callable

	func _physics_process(_delta: float) -> void:
		ended.call()


func _initialize() -> void:
	call_deferred("_begin")


func _begin() -> void:
	if DisplayServer.get_name() == "headless":
		printerr("P4 pacing needs a graphical display")
		quit(1)
		return
	DisplayServer.window_set_flag(DisplayServer.WINDOW_FLAG_ALWAYS_ON_TOP, true)
	steady = "--p4-steady" in OS.get_cmdline_user_args()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--p4-screenshot="):
			screenshot_path = arg.trim_prefix("--p4-screenshot=")
	match_scene = load("res://match.tscn").instantiate()
	root.add_child(match_scene)
	var probe := AfterMatch.new()
	probe.ended = _after_tick
	probe.process_physics_priority = 100
	root.add_child(probe)
	physics_frame.connect(_drive)
	RenderingServer.frame_post_draw.connect(_drawn)


func _drive() -> void:
	if match_scene.current.get("status", -1) != 0:
		return
	var sim_frame := int(match_scene.current.frame)
	if last_simulation_frame >= 0 and sim_frame != last_simulation_frame + 1:
		printerr("match failed to advance one authoritative frame")
		quit(1)
		return
	last_simulation_frame = sim_frame
	physics_count += 1
	var phase := physics_count % 64
	for event in match_scene.current.events:
		if event.kind == "Hit":
			combat_hits += 1
		elif event.kind == "Blocked":
			combat_blocks += 1
	tick_start = Time.get_ticks_usec()
	if steady:
		_hold(&"p1_guard", phase >= 32)
		_hold(&"p2_guard", phase < 32)
		_hold(&"p1_attack", phase in [4, 16, 28])
		_hold(&"p2_attack", phase in [36, 48, 60])
		return
	if phase == 0:
		var began := Time.get_ticks_usec()
		if not match_scene.restart():
			quit(1)
			return
		last_simulation_frame = 0
		if frames >= WARMUP:
			restart_costs.append(Time.get_ticks_usec() - began)
	if phase in [4, 16, 28, 40]:
		Input.action_press(&"p1_attack")
	else:
		Input.action_release(&"p1_attack")
	if phase == 16:
		Input.action_press(&"p2_attack")
	else:
		Input.action_release(&"p2_attack")
	if phase == 52:
		Input.action_press(&"p2_jump")
	else:
		Input.action_release(&"p2_jump")


func _after_tick() -> void:
	if tick_start != 0 and frames >= WARMUP:
		frame_cpu_us.append(Time.get_ticks_usec() - tick_start)


func _hold(action: StringName, pressed: bool) -> void:
	if pressed:
		Input.action_press(action)
	else:
		Input.action_release(action)


func _drawn() -> void:
	var now := Time.get_ticks_usec()
	frames += 1
	if frames < WARMUP and not screenshot_path.is_empty() and not match_scene.current.is_empty():
		if match_scene.current.fighters[0].state == "Attack" and match_scene.current.fighters[1].state == "Hitstun":
			if root.get_texture().get_image().save_png(screenshot_path) != OK:
				quit(1)
				return
			screenshot_path = ""
	if frames == WARMUP:
		start_ticks = physics_count
		start_hits = combat_hits
		start_blocks = combat_blocks
	if frames > WARMUP:
		ticks.append(now - previous)
	previous = now
	if ticks.size() != SAMPLES:
		return
	var total := 0
	var late := 0
	for t in ticks:
		total += t
		if t > 25000:
			late += 1
	ticks.sort()
	frame_cpu_us.sort()
	if frame_cpu_us.is_empty():
		quit(1)
		return
	var active_ticks := physics_count - start_ticks
	var active_hits := combat_hits - start_hits
	var active_blocks := combat_blocks - start_blocks
	print(JSON.stringify({"display":DisplayServer.get_name(),"renderer":RenderingServer.get_current_rendering_method(),
		"samples":SAMPLES,"elapsed_us":total,"fps":SAMPLES * 1000000.0 / total,
		"physics_span_p99_us":frame_cpu_us[ceili(frame_cpu_us.size() * 0.99) - 1],
		"physics_span_max_us":frame_cpu_us[-1],"physics_ticks":active_ticks,"hits":active_hits,"blocked":active_blocks,"steady":steady,"restart_us":Array(restart_costs),"p50_us":ticks[299],"p95_us":ticks[569],
		"p99_us":ticks[593],"max_us":ticks[-1],"intervals_over_25ms":late}))
	quit(0 if active_ticks > 0 and (active_hits + active_blocks > 0) and screenshot_path.is_empty() else 1)
