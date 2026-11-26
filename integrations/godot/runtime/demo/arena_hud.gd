# SPDX-License-Identifier: MPL-2.0
extends "res://match/hud.gd"

signal training_action(index: int)
const ACCENTS := [Color("#ffae65"), Color("#94d9e6"), Color("#e8c876"), Color("#beb2f5")]
const INK := Color("#192a31")
const PAPER := Color("#fff0cf")

var actor_tags: Array[Label] = []
var details: Array[Label] = []
var buttons: Array[Button] = []
var tape_label: Label
var mode_name: String
var previous_buttons: Array = []

func _ready() -> void:
	pass # The arena configures this HUD after it knows the roster.

func configure(mode: String, count: int) -> void:
	mode_name = mode
	_panel(0, 0, 960, 92, INK)
	var cell := 480 if count == 2 else 240
	for i in range(count):
		var x := i * cell + 16
		var accent: Color = ACCENTS[i]
		names.append(_label(x, 3, cell - 32, 27, 18, HORIZONTAL_ALIGNMENT_LEFT, PAPER))
		var bar := ProgressBar.new()
		bar.position = Vector2(x, 34)
		bar.show_percentage = false
		for style in ["background", "fill"]:
			var box := StyleBoxFlat.new()
			box.bg_color = Color("#46575c") if style == "background" else accent
			bar.add_theme_stylebox_override(style, box)
		add_child(bar)
		bar.size = Vector2(cell - 32, 7)
		bars.append(bar)
		values.append(_label(x, 43, cell - 32, 20, 13, HORIZONTAL_ALIGNMENT_LEFT, accent))
		var tag := _label(0, 0, 50, 20, 14, HORIZONTAL_ALIGNMENT_CENTER, PAPER, "P%d" % (i + 1))
		tag.add_theme_constant_override("outline_size", 4)
		tag.add_theme_color_override("font_outline_color", INK)
		actor_tags.append(tag)
	_label(16, 65, 430, 24, 13, HORIZONTAL_ALIGNMENT_LEFT, Color("#e8c876"),
		{"duel":"LOCAL DUEL", "team":"TEAM BATTLE", "ffa":"FREE FOR ALL", "training":"TRAINING"}[mode])
	clock_label = _label(336, 65, 288, 24, 14, HORIZONTAL_ALIGNMENT_CENTER, PAPER)
	_label(750, 65, 192, 24, 13, HORIZONTAL_ALIGNMENT_RIGHT, PAPER, "Esc   Pause")
	message = _label(30, 234, 900, 50, 21, HORIZONTAL_ALIGNMENT_CENTER, PAPER)
	message.add_theme_constant_override("outline_size", 4)
	if mode == "training":
		_training_controls()
	else:
		_panel(0, 499, 960, 41, INK)
		_label(16, 501, 928, 36, 15, HORIZONTAL_ALIGNMENT_CENTER, PAPER,
			"P1   F / E / Q   Attack · Special · Super         P2   J / I / U   Attack · Special · Super")

func _training_controls() -> void:
	_panel(0, 92, 960, 70, Color("#192a31ed"))
	for i in range(2):
		details.append(_label(16 + i * 480, 92, 455, 69, 13, HORIZONTAL_ALIGNMENT_LEFT, PAPER))
	_panel(0, 458, 960, 82, INK)
	tape_label = _label(16, 460, 930, 22, 13, HORIZONTAL_ALIGNMENT_LEFT, PAPER)
	var row := HBoxContainer.new()
	row.position = Vector2(12, 487)
	row.size = Vector2(936, 42)
	row.add_theme_constant_override("separation", 4)
	add_child(row)
	for i in range(11):
		var button := Button.new()
		button.focus_mode = Control.FOCUS_NONE
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.add_theme_font_size_override("font_size", 12)
		row.add_child(button)
		button.pressed.connect(func(): training_action.emit(i))
		buttons.append(button)

func present(snap: Dictionary) -> void:
	var seconds := floori(float(snap.frame) / 60.0)
	clock_label.text = "%02d:%02d" % [floori(seconds / 60.0), seconds % 60]
	for i in range(snap.fighters.size()):
		var f: Dictionary = snap.fighters[i]
		names[i].text = "P%d   %s%s" % [int(f.id), str(f.name).to_upper(),
			"   /   T%d" % int(f.team) if mode_name == "team" else ""]
		bars[i].max_value = f.max_hp
		bars[i].value = f.hp
		values[i].text = "HP  %d / %d      EN  %d" % [f.hp, f.max_hp, f.energy]
		if mode_name == "training":
			var move_name := str(f.move_id) if str(f.move_id) != "" else "—"
			details[i].text = "%s  ·  %s     X/Z/H  (%d, %d, %d)\nStartup %d    Active %d    Recovery %d    Hitstun %d    Blockstun %d\nCombo %d    Damage %d" % [
				f.state, move_name, f.x, f.z, f.h, f.startup, f.active, f.recovery,
				f.hitstun, f.blockstun, f.combo, f.damage]

func track(index: int, at: Vector2, alive: bool) -> void:
	actor_tags[index].position = Vector2(at.x - 25, at.y + 7)
	actor_tags[index].visible = alive

func training_status(snap: Dictionary, dummy: int, boxes: bool, infinite: bool, slow: bool,
		freeze: bool, rec: bool, replay: bool, length: int) -> void:
	var captions := [["Stand", "Guard", "After hit"][dummy], "Boxes " + ("on" if boxes else "off"),
		"HP " + ("∞" if infinite else "finite"), "Reset", "Speed " + ("¼" if slow else "1×"),
		"Resume" if freeze else "Freeze", "Step", "Stop" if rec else "Record", "Replay", "Save", "Load"]
	if captions != previous_buttons:
		for i in range(buttons.size()):
			buttons[i].text = "F%d\n%s" % [i + 1, captions[i]]
		previous_buttons = captions
	buttons[2].disabled = rec or replay
	buttons[8].disabled = length == 0
	buttons[9].disabled = rec or replay or length == 0
	var status := "Recording" if rec else ("Replaying" if replay else "Saved inputs")
	tape_label.text = "Frame  %d     ·     %s  %d frames     ·     Gold  push    Cyan  hurt    Coral  hit" % [
		int(snap.get("frame", 0)), status, length]
