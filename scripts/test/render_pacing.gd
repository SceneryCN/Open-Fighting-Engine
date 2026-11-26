# SPDX-License-Identifier: MPL-2.0
# Run against the installed P0 project using a graphical display and --max-fps 60.
extends SceneTree

const WARMUP := 120
const SAMPLES := 600
var ticks := PackedInt64Array()
var previous := 0
var frames := 0
var physics_start := 0
var proof: Node

func _initialize() -> void:
	call_deferred("_start")

func _start() -> void:
	if DisplayServer.get_name() == "headless":
		printerr("A graphical display is required for render timing")
		quit(1)
		return
	proof = load("res://proof.tscn").instantiate()
	root.add_child(proof)
	RenderingServer.frame_post_draw.connect(_drawn)

func _drawn() -> void:
	var now := Time.get_ticks_usec()
	frames += 1
	if frames == WARMUP:
		physics_start = proof.current.frame
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
	var physics_ticks: int = proof.current.frame - physics_start
	print(JSON.stringify({"display": DisplayServer.get_name(), "renderer": RenderingServer.get_current_rendering_method(),
		"samples": SAMPLES, "elapsed_us": total, "fps": SAMPLES * 1000000.0 / total,
		"physics_ticks": physics_ticks, "physics_hz": physics_ticks * 1000000.0 / total,
		"p50_us": ticks[299], "p95_us": ticks[569], "p99_us": ticks[593],
		"max_us": ticks[-1], "intervals_over_25ms": late}))
	quit(0 if physics_ticks > 0 else 1)
