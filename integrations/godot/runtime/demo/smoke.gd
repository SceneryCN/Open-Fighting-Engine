# SPDX-License-Identifier: MPL-2.0
extends RefCounted

# Run the same functional checks from both the editor project and release package.
var game
var tree: SceneTree
var error := ""

func run(root) -> void:
	game = root
	tree = root.get_tree()
	await _exercise()
	for action in [&"p1_attack", &"p1_right", &"p1_guard", &"p1_special", &"p1_super", &"p2_guard"]:
		if InputMap.has_action(action):
			Input.action_release(action)
	await game.stop_audio()
	if error.is_empty():
		print("OFE_P5_PASS")
	else:
		push_error("P5 acceptance: " + error)
	tree.quit(0 if error.is_empty() else 1)

func check(condition: bool, message: String) -> bool:
	if not condition and error.is_empty():
		error = message
	return condition

func _find_button(caption: String):
	for node in game.find_children("*", "BaseButton", true, false):
		if node.is_visible_in_tree() and node.text.nocasecmp_to(caption) == 0:
			return node
	return null

func press(caption: String) -> bool:
	var button = _find_button(caption)
	if not check(button != null, "missing visible button %s on %s" % [caption, game.screen]):
		return false
	if not check(not button.disabled, "disabled button " + caption):
		return false
	await _click(button)
	return true

func _click(button: BaseButton) -> void:
	var at := button.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = at
	motion.global_position = at
	tree.root.push_input(motion, true)
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		event.position = at
		event.global_position = at
		tree.root.push_input(event, true)
		await tree.process_frame

func _key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.physical_keycode = code
		event.keycode = code
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()
		await tree.process_frame

func _enter(caption: String, mode: String, teams: Array) -> bool:
	if not await press(caption) or not check(game.screen == "select", "character selection " + caption):
		return false
	if mode == "team":
		var picker := game.page.find_children("*", "OptionButton", true, false)[2] as OptionButton
		picker.select(1)
		picker.item_selected.emit(1)
	if not await press("Continue") or not check(game.screen == "vs", "VS screen " + caption):
		return false
	if not await press("Fight") or not check(game.screen == "battle", "battle screen " + caption):
		return false
	var battle = game.arena
	if not check(battle != null and battle.mode == mode, "mode " + mode):
		return false
	await tree.physics_frame
	await tree.physics_frame
	var snap: Dictionary = battle.current
	if not check(snap.get("status", -1) == 0 and snap.frame > 0 and snap.fighters.size() == teams.size(), "native roster " + mode):
		return false
	for slot in range(teams.size()):
		var actor: Dictionary = snap.fighters[slot]
		if not check(str(actor.name).to_lower() == game.roster[slot], "native actor differs from picker " + mode):
			return false
		if not check(actor.id == slot + 1 and actor.team == teams[slot], "native slot/team in " + mode):
			return false
		if not check(battle.sprites[slot].texture != null and battle.sprites[slot].is_visible_in_tree(), "undrawn fighter in " + mode):
			return false
	return true

func _pause() -> bool:
	await _key(KEY_ESCAPE)
	if not check(game.screen == "pause", "Escape did not pause battle"):
		return false
	var frame: int = game.arena.current.frame
	for tick in range(6):
		await tree.physics_frame
	return check(game.arena.current.frame == frame, "native frame advanced while paused")

func _exercise() -> void:
	if not check(game.screen == "menu", "default scene did not enter the main menu"):
		return
	if not await press("Settings") or not check(game.screen == "settings", "settings screen"):
		return
	var music_enabled := false
	for player in game.find_children("*", "AudioStreamPlayer", true, false):
		if player.bus == &"Music" and player.playing and player.stream is AudioStreamWAV:
			music_enabled = player.stream.loop_mode != AudioStreamWAV.LOOP_DISABLED
	if not check(music_enabled, "missing looping music"):
		return
	if not check(game.page.find_children("*", "HSlider", true, false).size() == 2, "separate music/effects settings"):
		return
	if not await press("Back"):
		return
	if not await _enter("Local duel", "duel", [1, 2]):
		return
	var battle = game.arena
	if not _native_coverage(battle._dirs):
		return
	if not await _pause() or not await press("Resume"):
		return
	var start_frame: int = battle.current.frame
	for tick in range(3):
		await tree.physics_frame
	if not check(game.screen == "battle" and battle.current.frame > start_frame, "resume did not advance native match"):
		return
	var strikes := 0
	var sounding_hit := false
	for tick in range(1500):
		if tick % 12 == 0:
			Input.action_press(&"p1_attack")
		elif tick % 12 == 1:
			Input.action_release(&"p1_attack")
		if game.screen == "battle":
			var fighters: Array = battle.current.fighters
			var distance: float = float(fighters[1].x - fighters[0].x)
			if distance > 4:
				Input.action_press(&"p1_right")
			else:
				Input.action_release(&"p1_right")
			for event in battle.current.events:
				if event.kind == "Hit":
					strikes += 1
					sounding_hit = sounding_hit or (battle.audio.playing and battle.audio.bus == &"SFX")
		await tree.physics_frame
		if game.screen == "result":
			break
	Input.action_release(&"p1_attack")
	Input.action_release(&"p1_right")
	if not check(strikes > 0 and sounding_hit and game.screen == "result" and game.result_caption.contains("P1"),
			"KO feedback: hits=%d audible=%s bus=%s screen=%s winner=%s" % [strikes, sounding_hit, battle.audio.bus, game.screen, game.result_caption]):
		return
	if not await press("Rematch") or not check(game.screen == "battle", "rematch"):
		return
	var fighter: Dictionary = game.arena.current.fighters[1]
	if not check(fighter.hp == fighter.max_hp, "rematch retained KO health"):
		return
	if not await _pause() or not await press("Main menu"):
		return
	for item in [["Team battle", "team", [1, 1, 2, 2]], ["Free-for-all", "ffa", [1, 2, 3, 4]]]:
		if not await _enter(item[0], item[1], item[2]):
			return
		if not await _pause() or not await press("Main menu"):
			return
	if not await _enter("Training", "training", [1, 2]):
		return
	await _training()

func _training() -> void:
	var battle = game.arena
	var fighter: Dictionary = battle.current.fighters[0]
	for field in ["move_id", "startup", "active", "recovery", "hitstun", "blockstun", "combo", "damage", "boxes"]:
		if not check(fighter.has(field), "training native snapshot missing " + field):
			return
	var original_dummy: int = battle.dummy_mode
	for variant in range(3):
		await _key(KEY_F1)
		if not check(battle.dummy_mode == (original_dummy + variant + 1) % 3, "dummy mode rotation"):
			return
	if not await _pause() or not await press("Resume"):
		return
	var visible_boxes: bool = battle.boxes_visible
	await _click(battle.get_node("HUD").buttons[1])
	if not check(battle.boxes_visible != visible_boxes, "collision overlay toggle"):
		return
	var initial_health_rule: bool = battle.infinite_health
	await _key(KEY_F3)
	if not check(battle.infinite_health != initial_health_rule, "infinite health toggle"):
		return
	await _key(KEY_F5)
	var slow_begin: int = battle.current.frame
	for tick in range(16):
		await tree.physics_frame
	var advanced: int = battle.current.frame - slow_begin
	if not check(advanced > 0 and advanced < 16, "slow motion did not reduce simulation rate"):
		return
	await _key(KEY_F5)
	await _key(KEY_F6)
	var frame: int = battle.current.frame
	for tick in range(6):
		await tree.physics_frame
	if not check(battle.current.frame == frame, "training freeze advanced"):
		return
	await _key(KEY_F7)
	for tick in range(2):
		await tree.physics_frame
	if not check(battle.current.frame == frame + 1, "single step should advance exactly one tick"):
		return
	await _key(KEY_F6)
	await _key(KEY_F4)
	await _key(KEY_F8)
	Input.action_press(&"p1_right")
	for tick in range(8):
		await tree.physics_frame
	Input.action_release(&"p1_right")
	await _key(KEY_F8)
	var recorded_x: int = battle.current.fighters[0].x
	await _key(KEY_F9)
	for tick in range(40):
		await tree.physics_frame
	if not check(battle.current.fighters[0].x == recorded_x, "recorded walk endpoint differs on playback"):
		return
	if not await _pause() or not await press("Main menu"):
		return
	check(game.screen == "menu", "return to menu after training")

func _native_coverage(dirs: PackedStringArray) -> bool:
	var native: RefCounted = ClassDB.instantiate(&"OFEMatch")
	if not check(native.open_roster(dirs, "ffa") != 0, "FFA accepted two actors"):
		return false
	if not check(native.open_roster(dirs, "training") == 0, "training open"):
		return false
	var before: Dictionary = native.snapshot()
	if not check(native.step_roster(1, PackedInt32Array([16])) != 0 and native.snapshot() == before, "partial roster changed frame"):
		return false
	if not check(native.step_roster(2, PackedInt32Array([16, 0])) != 0 and native.snapshot() == before, "wrong frame mutated match"):
		return false
	var names := []
	for move_index in [1, 2]:
		for slot in range(2):
			native.close()
			if not check(native.open_roster(dirs, "training") == 0, "reopen special test"):
				return false
			var masks := PackedInt32Array([0, 0])
			masks[slot] = (move_index + 1) << 8
			if not check(native.step_roster(1, masks) == 0, "move selection %d/%d" % [slot, move_index]):
				return false
			var source: Dictionary = native.snapshot().fighters[slot]
			if not check(not source.move_id.is_empty() and source.startup + source.active + source.recovery > 0, "no move data for special/super"):
				return false
			names.append(source.move_id)
	if not check(names[0] != names[1] and names[2] != names[3], "different characters share special or super"):
		return false
	native.close()
	if not check(native.open_roster(dirs, "training") == 0 and native.set_infinite_health(true) == 0, "enable infinite health"):
		return false
	var found_hit := false
	var found_hitbox := false
	for frame in range(1, 20):
		if not check(native.step_roster(frame, PackedInt32Array([16 if frame == 1 else 0, 0])) == 0, "training step"):
			return false
		var snap: Dictionary = native.snapshot()
		for box in snap.fighters[0].boxes:
			if box.kind == "hit" and box.width > 0 and box.height > 0:
				found_hitbox = true
		for event in snap.events:
			if event.kind == "Hit":
				found_hit = true
				if not check(event.damage > 0 and snap.fighters[0].damage == event.damage and snap.fighters[1].hp == before.fighters[1].hp, "infinite health lost event, health or damage"):
					return false
	native.close()
	return check(found_hit and found_hitbox, "native hit event or active training hitbox missing")
