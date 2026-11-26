# SPDX-License-Identifier: MPL-2.0
extends SceneTree

# Measure draws and main-thread simulation of the shipped four-fighter scene.
const WARMUP := 120
const SAMPLES := 600
var game
var arena
var tape_mode := ""
var training := false
var physics_ticks := 0
var frames := 0
var last_draw := 0
var tick_started := 0
var draw_us := PackedInt64Array()
var cpu_us := PackedInt64Array()
var attacks := 0

class TickProbe extends Node:
	var ended: Callable
	func _physics_process(_delta: float) -> void:
		ended.call()

func _initialize() -> void:
	call_deferred("_start")

func _start() -> void:
	if DisplayServer.get_name() == "headless":
		printerr("Graphical display required")
		quit(1)
		return
	var args := OS.get_cmdline_user_args()
	tape_mode = "record" if "--record" in args else ("replay" if "--replay" in args else "")
	training = "--training" in args or not tape_mode.is_empty()
	game = load("res://demo.tscn").instantiate()
	root.add_child(game)
	game._choose_mode("training" if training else "ffa")
	game._versus()
	game._fight()
	arena = game.arena
	if training:
		arena._training_action(1)
	if not tape_mode.is_empty():
		arena._training_action(7)
		if tape_mode == "replay":
			for tick in range(1000):
				if tick % 48 == 1:
					Input.action_press(&"p1_attack")
				elif tick % 48 == 4:
					Input.action_release(&"p1_attack")
				arena._physics_process(1.0/60)
			Input.action_release(&"p1_attack")
			arena._training_action(7)
			arena._training_action(8)
	if arena.current.get("status", -1) != 0:
		quit(1)
		return
	var probe := TickProbe.new()
	probe.ended = _after
	probe.process_physics_priority = 100
	root.add_child(probe)
	physics_frame.connect(_before)
	RenderingServer.frame_post_draw.connect(_drawn)

func _before() -> void:
	tick_started = Time.get_ticks_usec()
	physics_ticks += 1
	for event in arena.current.events:
		if event.kind == "Started":
			attacks += 1
	for slot in range(1, 2 if training else 5):
		var phase := (physics_ticks + slot * 9) % 48
		for binding in [["attack", phase == 8], ["guard", phase < 7 or (phase >= 12 and phase < 37)], ["lane", phase == 40]]:
			var action := StringName("p%d_%s" % [slot, binding[0]])
			if binding[1]:
				Input.action_press(action)
			else:
				Input.action_release(action)

func _after() -> void:
	if tick_started and frames >= WARMUP:
		cpu_us.append(Time.get_ticks_usec() - tick_started)

func _drawn() -> void:
	var now := Time.get_ticks_usec()
	frames += 1
	if frames == WARMUP / 2:
		for arg in OS.get_cmdline_user_args():
			if arg.begins_with("--capture="):
				var path := arg.trim_prefix("--capture=")
				if root.get_texture().get_image().save_png(path) != OK:
					quit(1)
					return
	if frames > WARMUP:
		draw_us.append(now - last_draw)
	last_draw = now
	if draw_us.size() != SAMPLES:
		return
	var total := 0
	var late := 0
	for duration in draw_us:
		total += duration
		if duration > 25000:
			late += 1
	draw_us.sort()
	cpu_us.sort()
	print(JSON.stringify({"display":DisplayServer.get_name(), "renderer":RenderingServer.get_current_rendering_method(),
		"mode":arena.mode, "tape":tape_mode, "samples":SAMPLES, "physics_samples":cpu_us.size(), "frame":arena.current.frame,
		"fps":1000000.0*SAMPLES/total, "draw_p95_us":draw_us[569], "draw_p99_us":draw_us[593],
		"draw_max_us":draw_us[-1], "over_25ms":late, "physics_p99_us":cpu_us[ceili(cpu_us.size()*0.99)-1],
		"physics_max_us":cpu_us[-1], "actions_started":attacks}))
	RenderingServer.frame_post_draw.disconnect(_drawn)
	physics_frame.disconnect(_before)
	call_deferred("_finish", 0 if attacks > 0 and arena.current.frame >= SAMPLES else 1)

func _finish(code: int) -> void:
	await game.stop_audio()
	quit(code)
