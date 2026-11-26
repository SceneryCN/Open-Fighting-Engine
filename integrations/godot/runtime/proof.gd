# SPDX-License-Identifier: MPL-2.0
extends Node2D

@export var depth_projection: float = 0.15
var battle: RefCounted
var current: Dictionary
var smoke := false


func _ready() -> void:
	smoke = "--p0-test" in OS.get_cmdline_user_args()
	if not ClassDB.class_exists(&"OFEBattle"):
		_fail("Extension class OFEBattle missing")
		return
	if smoke:
		set_physics_process(false)
		if _run_smoke():
			print("OFE_P0_PASS")
			get_tree().quit(0)
		return
	battle = ClassDB.instantiate(&"OFEBattle")
	if battle.open() != 0:
		_fail("create failed")
		return
	current = battle.read_state()
	if current.status != 0:
		_fail("initial read failed")
		return
	_present()


func _physics_process(_delta: float) -> void:
	if smoke or battle == null:
		return
	var axis := int(Input.is_action_pressed("ui_right")) - int(Input.is_action_pressed("ui_left"))
	if battle.submit(current.frame + 1, axis) != 0 or battle.step() != 0:
		_fail("input or step failed")
		return
	current = battle.read_state()
	if current.status != 0:
		_fail("state read failed")
		return
	_present()


func _present() -> void:
	$Readout.text = "OFE   /   Bridge proof\nLeft / Right to move\nFrame %d    X %d    Z %d    H %d" % [
		current.frame, current.x, current.z, current.h
	]
	queue_redraw()


func _draw() -> void:
	if current.get("status", -1) != 0:
		return
	var ground := Vector2(320, 250)
	draw_line(Vector2(0, ground.y), Vector2(640, ground.y), Color.SLATE_GRAY)
	var point := ground + Vector2(current.x, -depth_projection * current.z - current.h)
	draw_circle(point, 7, Color.AQUAMARINE)


func _fail(message: String) -> bool:
	push_error(message)
	set_physics_process(false)
	get_tree().quit(1)
	return false


func _run_smoke() -> bool:
	for index in range(256):
		var actor: RefCounted = ClassDB.instantiate(&"OFEBattle")
		if actor.open() != 0 or actor.open() != 1:
			return _fail("lifecycle %d: open" % index)
		var before: Dictionary = actor.read_state()
		if before != {"status": 0, "frame": 0, "entity_id": 1, "x": 0, "z": 0, "h": 0}:
			return _fail("unexpected initial state: %s" % before)
		if actor.submit(1, 2) != 1 or actor.submit(2, 1) != 4:
			return _fail("invalid input accepted")
		if actor.submit(1, -1) != 0 or actor.submit(1, 1) != 4 or actor.step() != 0:
			return _fail("submit or step")
		var after: Dictionary = actor.read_state()
		if after.status != 0 or after.frame != 1 or after.x != -1:
			return _fail("unexpected state after step: %s" % after)
		if index % 2 == 0:
			if actor.close() != 0 or actor.close() != 2 or actor.read_state().status != 2:
				return _fail("destroy or stale handle")
		# Remaining cases release through the C++ RefCounted destructor.
	return true


func _exit_tree() -> void:
	if battle != null and battle.close() != 0:
		push_error("OFE battle release failed")
	battle = null
	current.clear()
	request_ready()
