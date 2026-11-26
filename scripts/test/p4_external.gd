# SPDX-License-Identifier: MPL-2.0
extends SceneTree


func _initialize() -> void:
	call_deferred("_verify")


func _verify() -> void:
	var arena = load("res://match.tscn").instantiate()
	root.add_child(arena)
	await physics_frame
	if arena.current.get("status", -1) != 0 or arena.current.fighters[0].max_hp != 137 or arena.current.fighters[1].max_hp != 191:
		printerr("external OFC pair did not supply independent starting health")
		quit(1)
		return
	Input.action_press(&"p1_attack")
	await physics_frame
	Input.action_release(&"p1_attack")
	for tick in range(10):
		await physics_frame
		if arena.current.fighters[1].hp == 168 and arena.health_bars[1].value == 168:
			await arena.audio.finished
			arena.queue_free()
			await process_frame
			print("OFE_P4_EXTERNAL_PASS")
			quit(0)
			return
	printerr("expected 23 damage from external first player's move")
	quit(1)
