# SPDX-License-Identifier: MPL-2.0
extends RefCounted

# Exercise the exported default scene through the same mapped input as a player.
func run(scene) -> void:
	var tree: SceneTree = scene.get_tree()
	var error: String = await _exercise(scene, tree)
	for action in [&"p1_attack", &"p1_right", &"p1_jump", &"p2_guard"]:
		Input.action_release(action)
	if not error.is_empty():
		push_error("P4 acceptance: " + error)
		tree.quit(1)
		return
	print("OFE_P4_PASS")
	tree.quit(0)


func _presentation(scene) -> String:
	var fighters: Array = scene.current.get("fighters", [])
	if fighters.size() != 2 or scene.sprites.size() != 2 or scene.health_bars.size() != 2:
		return "expected two native fighters, sprites and HUD bars"
	for index in range(2):
		var fighter: Dictionary = fighters[index]
		var animation: Dictionary = fighter.animation
		var area: Array = animation.rect
		var sprite: Sprite2D = scene.sprites[index]
		var bar: Range = scene.health_bars[index]
		if not sprite.is_visible_in_tree() or sprite.texture == null or not sprite.region_enabled:
			return "invisible or untextured fighter %d" % index
		if sprite.region_rect != Rect2(area[0], area[1], area[2], area[3]):
			return "displayed texture region differs from OFA for fighter %d" % index
		if not bar.is_visible_in_tree() or bar.value != fighter.hp or bar.max_value != fighter.max_hp:
			return "health bar differs from native health for fighter %d" % index
	return ""


func _exercise(scene, tree: SceneTree) -> String:
	await tree.physics_frame
	if not scene.restart() or scene.current.get("frame", -1) != 0:
		return "restart failed to construct frame zero"
	var error := _presentation(scene)
	if not error.is_empty():
		return error
	var initial_hp: int = int(scene.current.fighters[1].hp)
	var initial_x: int = int(scene.current.fighters[0].x)
	var initial_pose: String = scene.current.fighters[0].animation.clip
	var sound_before: int = scene.sound_count
	var hit := false
	var advanced_pose := false
	var heard := false
	Input.action_press(&"p1_attack")
	for tick in range(12):
		await tree.physics_frame
		Input.action_release(&"p1_attack")
		var state: Dictionary = scene.current
		for event in state.events:
			if event.kind == "Hit":
				hit = true
				heard = scene.audio.stream != null and scene.audio.playing and scene.sound_count > sound_before
		if state.fighters[0].animation.clip != initial_pose or state.fighters[0].animation.index != 0:
			advanced_pose = true
		error = _presentation(scene)
		if not error.is_empty():
			return error
	if not hit or scene.current.fighters[1].hp >= initial_hp or not heard:
		return "mapped attack did not produce native hit, corresponding HP loss and playing audio"
	if not advanced_pose:
		return "OFA clip/frame did not advance"
	if not scene.restart():
		return "failed to reset after hit"
	Input.action_press(&"p2_guard")
	Input.action_press(&"p1_attack")
	var blocked := false
	for tick in range(12):
		await tree.physics_frame
		Input.action_release(&"p1_attack")
		for event in scene.current.events:
			blocked = blocked or event.kind == "Blocked"
	Input.action_release(&"p2_guard")
	if not blocked or scene.current.fighters[1].hp != initial_hp:
		return "player-two guard failed to block the hit"
	if not scene.restart():
		return "failed to reset after guard"
	var screen_x: float = scene.sprites[0].global_position.x
	Input.action_press(&"p1_right")
	await tree.physics_frame
	Input.action_release(&"p1_right")
	if scene.current.fighters[0].x <= initial_x or scene.sprites[0].global_position.x <= screen_x:
		return "mapped movement did not move both simulation and sprite"
	var screen_y: float = scene.sprites[0].global_position.y
	Input.action_press(&"p1_jump")
	await tree.physics_frame
	Input.action_release(&"p1_jump")
	if scene.current.fighters[0].h <= 0 or scene.sprites[0].global_position.y >= screen_y:
		return "mapped jump failed to project native height"
	var old_instance: RefCounted = scene.match_instance
	var owner: Node = scene.get_parent()
	owner.remove_child(scene)
	if scene.match_instance != null or not scene.textures.is_empty() or old_instance.snapshot().status == 0:
		return "exit retained native handle or texture cache"
	owner.add_child(scene)
	await tree.physics_frame
	await tree.physics_frame
	if scene.match_instance == null or scene.current.get("frame", 0) < 1:
		return "re-entered scene failed to advance"
	if scene.current.fighters[1].hp != initial_hp:
		return "re-entered scene retained previous damage"
	return _presentation(scene)
