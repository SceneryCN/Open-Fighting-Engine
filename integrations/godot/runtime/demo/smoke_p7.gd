# SPDX-License-Identifier: MPL-2.0
extends "res://demo/smoke.gd"

func run(root) -> void:
	game = root
	tree = game.get_tree()
	await _exercise_project()
	for action in [&"p1_attack", &"p1_right"]:
		if InputMap.has_action(action):
			Input.action_release(action)
	await game.stop_audio()
	if error.is_empty():
		print("OFE_P7_PASS")
	else:
		push_error("P7 acceptance: " + error)
	tree.quit(0 if error.is_empty() else 1)

func _label_visible(caption: String) -> bool:
	for label in game.page.find_children("*", "Label", true, false):
		if label.is_visible_in_tree() and label.text == caption:
			return true
	return false

func _select_fighter(slot: int, index: int) -> bool:
	var pickers = game.page.find_children("*", "OptionButton", true, false)
	if not check(pickers.size() > slot, "fighter selector missing"):
		return false
	var picker: OptionButton = pickers[slot]
	if not check(picker.item_count == 3 and picker.get_item_text(1) == "QA Rook" and picker.get_item_text(2) == "QA Echo", "mod display names missing from selector"):
		return false
	picker.select(index)
	picker.item_selected.emit(index)
	await tree.process_frame
	return true

func _exercise_project() -> void:
	if not check(game.screen == "menu" and _label_visible("QA Open Dojo"), "external project title missing from main menu"):
		return
	if not check(game.choices == PackedStringArray(["amber", "rook", "qa_echo"]), "project and mod ordering"):
		return
	if not check(_find_button("Training") == null and _find_button("Team battle") == null and _find_button("Free-for-all") == null, "project mode filtering"):
		return
	if not await press("Local duel") or not await _select_fighter(0, 1) or not await _select_fighter(1, 2):
		return
	if not await press("Continue") or not check(_label_visible("QA Rook") and _label_visible("QA Echo"), "VS screen names"):
		return
	if not await press("Fight") or not check(game.screen == "battle", "mod battle did not start"):
		return
	var battle = game.arena
	await tree.physics_frame
	await tree.physics_frame
	if not check(battle.current.get("status", -1) == 0 and battle.current.fighters.size() == 2, "native match startup"):
		return
	var actor: Dictionary = battle.current.fighters[0]
	var rival: Dictionary = battle.current.fighters[1]
	if not check(actor.name == "QA Rook" and actor.max_hp == 137 and rival.name == "QA Echo" and rival.max_hp == 113, "native OFC attributes"):
		return
	if not check(is_equal_approx(battle.depth_projection, 0.067), "project depth projection"):
		return
	if not check(Vector2(actor.animation.origin[0], actor.animation.origin[1]) == Vector2(9, 17) and battle.sprites[0].offset == Vector2(-9, -17)
			and battle.sprites[0].texture != null and battle.sprites[0].is_visible_in_tree(), "external OFA sprite origin=%s offset=%s texture=%s visible=%s" % [actor.animation.origin, battle.sprites[0].offset, battle.sprites[0].texture, battle.sprites[0].is_visible_in_tree()]):
		return
	var started := false
	var hit := false
	for tick in range(240):
		var fighters: Array = battle.current.fighters
		if float(fighters[1].x - fighters[0].x) > 3:
			Input.action_press(&"p1_right")
		else:
			Input.action_release(&"p1_right")
		if tick % 18 == 0:
			Input.action_press(&"p1_attack")
		elif tick % 18 == 2:
			Input.action_release(&"p1_attack")
		for event in battle.current.events:
			if event.kind == "Started":
				started = true
			if event.kind == "Hit" and event.attacker == 1 and event.target == 2:
				hit = check(event.damage == 23 and fighters[1].hp == 90 and battle.health_bars[1].value == 90, "custom move damage or HUD mismatch")
				break
		if hit or not error.is_empty():
			break
		await tree.physics_frame
	Input.action_release(&"p1_attack")
	Input.action_release(&"p1_right")
	if not check(started and hit, "no third-party attack/hit from player input"):
		return
	print("OFE_P7_BATTLE actors=QA_Rook,QA_Echo hp=137,90 damage=23 origin=9,17")
	if "--qa-pacing" in OS.get_cmdline_user_args():
		await _measure_pacing(battle)

func _measure_pacing(battle) -> void:
	if not check(DisplayServer.get_name() != "headless", "graphical display required for pacing"):
		return
	for i in range(120):
		await RenderingServer.frame_post_draw
	var intervals: Array[int] = []
	var previous := Time.get_ticks_usec()
	var start: int = battle.current.frame
	var total := 0
	var over_25ms := 0
	for i in range(600):
		if i % 60 == 0:
			Input.action_press(&"p1_guard")
		elif i % 60 == 30:
			Input.action_release(&"p1_guard")
		await RenderingServer.frame_post_draw
		var now := Time.get_ticks_usec()
		var elapsed := now - previous
		total += elapsed
		intervals.append(elapsed)
		if elapsed > 25000:
			over_25ms += 1
		previous = now
	Input.action_release(&"p1_guard")
	intervals.sort()
	print("OFE_P7_PACING ", JSON.stringify({"display":DisplayServer.get_name(), "samples":intervals.size(),
		"fps":600000000.0 / total, "draw_p95_us":intervals[569], "draw_p99_us":intervals[593],
		"draw_max_us":intervals[-1], "over_25ms":over_25ms, "simulation_ticks":battle.current.frame - start}))
	check(battle.current.frame > start, "native simulation did not advance while rendering")
