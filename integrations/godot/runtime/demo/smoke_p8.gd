# SPDX-License-Identifier: MPL-2.0
extends "res://demo/smoke.gd"

# Expectations are independent of the runtime bindings. Each key is delivered
# through Godot's input pipeline before reading the four native command masks.
const LAYOUT := [
	[KEY_A, KEY_D, KEY_W, KEY_S, KEY_J, KEY_K, KEY_L, KEY_U, KEY_I],
	[KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN, KEY_F, KEY_G, KEY_H, KEY_R, KEY_T],
	[KEY_Z, KEY_C, KEY_X, KEY_V, KEY_B, KEY_N, KEY_M, KEY_COMMA, KEY_PERIOD],
	[KEY_KP_4, KEY_KP_6, KEY_KP_8, KEY_KP_2, KEY_KP_1, KEY_KP_3, KEY_KP_5, KEY_KP_7, KEY_KP_9],
]
const MASKS := [1, 2, 4, 8, 16, 32, 64, 512, 768]

func run(root) -> void:
	game = root
	tree = root.get_tree()
	if await _enter("Free-for-all", "ffa", [1, 2, 3, 4]):
		_exercise_keys()
		if error.is_empty():
			await _exercise_moves()
	await game.stop_audio()
	if error.is_empty():
		print("OFE_P8_KEYS_PASS")
	else:
		push_error("P8 keyboard acceptance: " + error)
	tree.quit(0 if error.is_empty() else 1)

func _physical(code: Key, down: bool) -> void:
	var event := InputEventKey.new()
	event.physical_keycode = code
	event.pressed = down
	Input.parse_input_event(event)
	Input.flush_buffered_events()

func _expect_masks(expected: Array, detail: String) -> bool:
	var actual := []
	for seat in range(4):
		actual.append(game.arena._input_mask(seat))
	return check(actual == expected, "%s: expected %s, got %s" % [detail, expected, actual])

func _one_key(code: Key, seat: int, mask: int) -> void:
	var expected := [0, 0, 0, 0]
	expected[seat] = mask
	_physical(code, true)
	_expect_masks(expected, OS.get_keycode_string(code))
	_physical(code, false)
	_expect_masks([0, 0, 0, 0], "release " + OS.get_keycode_string(code))

func _exercise_keys() -> void:
	for seat in range(4):
		for index in range(MASKS.size()):
			_one_key(LAYOUT[seat][index], seat, MASKS[index])
	_one_key(KEY_O, 0, 16)
	var chord := [KEY_D, KEY_K, KEY_F, KEY_COMMA, KEY_KP_5]
	for code in chord:
		_physical(code, true)
	_expect_masks([34, 16, 512, 64], "simultaneous four-seat input")
	for code in chord:
		_physical(code, false)
	_expect_masks([0, 0, 0, 0], "release simultaneous inputs")

func _exercise_moves() -> void:
	for sample in [[KEY_J, 0, 0], [KEY_O, 0, 0], [KEY_U, 0, 1],
			[KEY_I, 0, 2], [KEY_F, 1, 0], [KEY_R, 1, 1], [KEY_T, 1, 2]]:
		var battle = game.arena
		if not check(battle.restart(), "could not reset fight"):
			return
		_physical(sample[0], true)
		var found := false
		for tick in range(5):
			await tree.physics_frame
			for event in battle.current.events:
				if event.kind == "Started" and event.id == sample[1] + 1 and event.move_index == sample[2]:
					found = true
			if found:
				break
		_physical(sample[0], false)
		if not check(found, "native move for %s must be %s" % [OS.get_keycode_string(sample[0]), sample[2]]):
			return
	print("OFE_P8_INPUT distinct_keys=37 seats=4 native_move_cases=7")
