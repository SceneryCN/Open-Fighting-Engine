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
var screenshot_path := ""
var restart_costs := PackedInt64Array()


func _initialize() -> void:
	call_deferred("_begin")


func _begin() -> void:
	if DisplayServer.get_name() == "headless":
		printerr("P4 pacing needs a graphical display")
		quit(1)
		return
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--p4-screenshot="):
			screenshot_path = arg.trim_prefix("--p4-screenshot=")
	match_scene = load("res://match.tscn").instantiate()
	root.add_child(match_scene)
	physics_frame.connect(_drive)
	RenderingServer.frame_post_draw.connect(_drawn)


func _drive() -> void:
	if match_scene.current.get("status", -1) != 0:
		return
	physics_count += 1
	var phase := physics_count % 64
	for event in match_scene.current.events:
		if event.kind == "Hit":
			combat_hits += 1
	if phase == 0:
		var began := Time.get_ticks_usec()
		if not match_scene.restart():
			quit(1)
			return
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
	var active_ticks := physics_count - start_ticks
	var active_hits := combat_hits - start_hits
	print(JSON.stringify({"display":DisplayServer.get_name(),"renderer":RenderingServer.get_current_rendering_method(),
		"samples":SAMPLES,"elapsed_us":total,"fps":SAMPLES * 1000000.0 / total,
		"physics_ticks":active_ticks,"hits":active_hits,"restart_us":Array(restart_costs),"p50_us":ticks[299],"p95_us":ticks[569],
		"p99_us":ticks[593],"max_us":ticks[-1],"intervals_over_25ms":late}))
	quit(0 if active_ticks > 0 and active_hits > 0 and screenshot_path.is_empty() else 1)
