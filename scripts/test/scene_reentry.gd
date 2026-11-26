# SPDX-License-Identifier: MPL-2.0
extends SceneTree


func _initialize() -> void:
	call_deferred("_check")


func _check() -> void:
	var scene = load("res://proof.tscn").instantiate()
	for cycle in range(3):
		root.add_child(scene)
		await physics_frame
		await physics_frame
		if scene.battle == null or scene.current.status != 0 or scene.current.frame < 1:
			push_error("Battle failed to advance on scene entry %d" % cycle)
			quit(1)
			return
		root.remove_child(scene)
		if scene.battle != null:
			push_error("Battle reference survived scene exit")
			quit(1)
			return
	scene.free()
	print("OFE_REENTRY_PASS")
	quit()
