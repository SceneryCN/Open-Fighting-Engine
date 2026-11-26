# SPDX-License-Identifier: MPL-2.0
extends Node2D

const INK := Color("#1a2c33")
const PAPER := Color("#fff0cf")
const COPPER := Color("#ffad70")
const MIST := Color("#a4d5db")
const MODES := {"duel":"Local duel", "team":"Team battle", "ffa":"Free-for-all", "training":"Training"}
const FIGHTERS := ["amber", "indigo"]

var screen := ""
var mode := "duel"
var roster := PackedStringArray()
var arena
var page: Control
const DemoSound = preload("res://demo/sound.gd")
var sound: DemoSound
var portraits: Dictionary = {}
var test_driver: RefCounted
var result_caption := ""
var external_dirs: Dictionary = {}
var controls_from_pause := false

func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	if "--p0-test" in args or "--p4-test" in args:
		var legacy := "res://proof.tscn" if "--p0-test" in args else "res://match.tscn"
		get_tree().change_scene_to_file.call_deferred(legacy)
		return
	for arg in args:
		for i in range(4):
			var prefix := "--p%d-dir=" % (i + 1)
			if arg.begins_with(prefix):
				external_dirs[i] = arg.trim_prefix(prefix).trim_suffix("/").get_file()
	get_tree().auto_accept_quit = false
	sound = DemoSound.new()
	add_child(sound)
	for id in FIGHTERS:
		var pack := ZIPReader.new()
		if pack.open("res://match/%s.ofc" % id) != OK:
			continue
		var sheet := Image.new()
		if sheet.load_png_from_buffer(pack.read_file("animations/assets/fighter.png")) == OK:
			var portrait := AtlasTexture.new()
			portrait.atlas = ImageTexture.create_from_image(sheet)
			portrait.region = Rect2(0, 0, sheet.get_width() / 2.0, sheet.get_height())
			portraits[id] = portrait
		pack.close()
	_main_menu()
	if "--p5-test" in args:
		test_driver = load("res://demo/smoke.gd").new()
		test_driver.run.call_deferred(self)

	elif "--p6-test" in args:
		test_driver = load("res://demo/smoke_p6.gd").new()
		test_driver.run.call_deferred(self)

func _page(id: String) -> void:
	if page != null:
		remove_child(page)
		page.queue_free()
	screen = id
	page = Control.new()
	page.size = Vector2(960, 540)
	page.mouse_filter = Control.MOUSE_FILTER_IGNORE
	page.z_index = 100
	add_child(page)

func _rect(x: int, y: int, w: int, h: int, tint: Color) -> void:
	var node := ColorRect.new()
	node.position = Vector2(x,y)
	node.size = Vector2(w,h)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.color = tint
	page.add_child(node)

func _label(caption: String, x: int, y: int, size_px: int, color := PAPER) -> Label:
	var label := Label.new()
	label.text = caption
	label.position = Vector2(x,y)
	label.add_theme_font_size_override("font_size", size_px)
	label.add_theme_color_override("font_color",color)
	page.add_child(label)
	return label

func _box(fill: Color, line: Color, focus := false) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = fill
	style.border_color = line
	style.border_width_left = 4
	if focus:
		style.draw_center = false
		style.set_border_width_all(2)
	style.content_margin_left = 14
	style.content_margin_right = 14
	return style

func _button(caption: String, x: int, y: int, w: int, action: Callable, focus := false) -> Button:
	var button := Button.new()
	button.text = caption
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.position = Vector2(x,y)
	button.custom_minimum_size = Vector2(w, 40)
	button.add_theme_font_size_override("font_size",18)
	button.add_theme_stylebox_override("normal", _box(Color("#283c43"), MIST))
	for state in ["hover","pressed"]:
		button.add_theme_stylebox_override(state, _box(Color("#48555a"), COPPER))
	button.add_theme_stylebox_override("focus", _box(INK,COPPER,true))
	page.add_child(button)
	button.pressed.connect(action)
	if focus:
		button.grab_focus()
	return button

func _portrait(id: String, x: int, y: int, side: int, flip := false) -> void:
	if not portraits.has(id):
		return
	var view := TextureRect.new()
	view.texture = portraits[id]
	view.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	view.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	view.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	view.mouse_filter = Control.MOUSE_FILTER_IGNORE
	view.position = Vector2(x,y)
	view.size = Vector2(side,side)
	view.flip_h = flip
	page.add_child(view)

func _main_menu() -> void:
	if is_instance_valid(arena):
		remove_child(arena)
		arena.queue_free()
		arena = null
	_page("menu")
	_rect(0,0,435,540, INK)
	_label("OPEN FIGHTING ENGINE", 42,38,19,MIST)
	_label("Sunset Viaduct", 40,71,46)
	var y := 168
	for id in MODES:
		_button(MODES[id],44,y,335,_choose_mode.bind(id),id == "duel")
		y += 52
	_button("Settings",44,y,335,_settings)
	_button("Controls",44,y+52,335,_controls.bind(false))
	_portrait("amber",414,132,316)
	_portrait("indigo",670,146,295,true)

func _choose_mode(id: String) -> void:
	mode = id
	roster = PackedStringArray(["amber","indigo"])
	if id in ["team","ffa"]:
		roster.append_array(PackedStringArray(["amber","indigo"]))
	_selection()

func _slot_name(i: int) -> String:
	if mode == "training" and i == 1:
		return "DUMMY"
	if mode == "team":
		return "TEAM %d   /   P%d" % [1 if i < 2 else 2, i+1]
	return "PLAYER %d" % (i+1)

func _selection(slot := 0) -> void:
	_page("select")
	_rect(0,0,960,110,INK)
	_label(str(MODES[mode]).to_upper(),32,12,16,MIST)
	_label("Select fighters",30,33,40)
	var width := 896 / roster.size()
	for i in range(roster.size()):
		var x := 32 + i * width
		var accent := COPPER if roster[i] == "amber" else MIST
		_rect(x,139,width-10,272,Color("#23373eeb"))
		_rect(x,139,width-10,4,accent)
		_label(_slot_name(i),x+10,150,15,accent)
		if external_dirs.has(i):
			_label("External OFC",x+10,244,19)
			_label(str(external_dirs[i]).left(18),x+10,278,14,accent)
			continue
		_portrait(roster[i],x+(width-152)/2,170,152, i % 2 == 1)
		var picker := OptionButton.new()
		_label("Rising strike" if roster[i] == "amber" else "Projectile specialist",x+10,323,14,accent)
		picker.position = Vector2(x+10,353)
		picker.custom_minimum_size = Vector2(width-30,42)
		for fighter in FIGHTERS:
			picker.add_item(str(fighter).capitalize())
		picker.selected = FIGHTERS.find(roster[i])
		picker.item_selected.connect(func(index): roster[i] = FIGHTERS[index]; _selection(i))
		page.add_child(picker)
		if i == slot:
			picker.grab_focus()
	_footer(_main_menu,"Continue",_versus)

func _footer(back: Callable, caption: String, next: Callable) -> void:
	_rect(0,449,960,91,INK)
	_button("Back",32,472,148,back)
	_button(caption,696,472,232,next)

func _versus() -> void:
	_page("vs")
	_rect(0,0,960,105,INK)
	_label("SUNSET VIADUCT",32,12,16,MIST)
	_label(str(MODES[mode]),30,32,39)
	var width := 920 / roster.size()
	for i in range(roster.size()):
		var x := 20 + width*i
		var side := mini(width-12,238)
		if not external_dirs.has(i):
			_portrait(roster[i],x+(width-side)/2,135,side,i>=roster.size()/2)
		_rect(x,363,width-4,67,INK)
		_label(_slot_name(i),x+12,367,13,MIST)
		_label(str(external_dirs.get(i,roster[i].capitalize())).left(18),x+12,383,25)
	_footer(_selection,"Fight",_fight)
	page.get_child(-1).grab_focus()

func _fight() -> void:
	_page("battle")
	if is_instance_valid(arena):
		arena.set_suspended(false)
		arena.restart()
		return
	arena = load("res://demo/arena.tscn").instantiate()
	arena.mode = mode
	arena.roster = roster.duplicate()
	arena.pause_requested.connect(_pause)
	arena.finished.connect(_result)
	add_child(arena)

func _pause() -> void:
	if screen != "battle":
		return
	arena.set_suspended(true)
	_overlay("pause","Paused")
	_pause_buttons()

func _pause_buttons() -> void:
	_button("Resume",330,209,300,_resume,true)
	_button("Restart",330,256,300,_fight)
	_button("Controls",330,303,300,_controls.bind(true))
	_button("Main menu",330,350,300,_main_menu)

func _resume() -> void:
	_page("battle")
	arena.set_suspended(false)

func _result(caption: String) -> void:
	result_caption = caption
	arena.set_suspended(true)
	_overlay("result",caption)
	_button("Rematch",330,254,300,_fight,true)
	_button("Main menu",330,306,300,_main_menu)

func _overlay(id: String, heading: String) -> void:
	_page(id)
	_rect(0,0,960,540,Color("#101c24bb"))
	_rect(200,114,560,304,INK)
	_rect(200,114,560,5,COPPER)
	_label(heading,241,146,34)

func _controls(in_match: bool) -> void:
	controls_from_pause = in_match
	_page("controls")
	_rect(0,0,960,540,INK)
	_label("Controls",34,21,40)
	var columns := [34,270,432,594,756]
	var rows := [
		["Action", "Player 1", "Player 2", "Player 3", "Player 4"],
		["Move", "A / D", "← / →", "Z / C", "Num 4 / 6"],
		["Jump / Change lane", "W / S", "↑ / ↓", "X / V", "Num 8 / 2"],
		["Attack / Guard", "F / G", "J / K", "B / N", "Num 1 / 3"],
		["Dash", "H", "L", "M", "Num 5"],
		["Special / Super", "E / Q", "I / U", ", / .", "Num 7 / 9"]
	]
	for r in range(rows.size()):
		for c in range(columns.size()):
			_label(rows[r][c],columns[c],91+r*40,17,MIST if r == 0 else PAPER)
	_label("Controller   ·   D-pad to move, jump and change lane",34,354,17,MIST)
	_label("A  Attack     B  Guard     LB  Dash     X  Special     Y  Super",34,389,18)
	_button("Back",34,463,183,_controls_back,true)

func _controls_back() -> void:
	if controls_from_pause:
		_overlay("pause","Paused")
		_pause_buttons()
	else:
		_main_menu()

func _settings() -> void:
	_page("settings")
	_rect(0,0,550,540,INK)
	_label("Settings",40,48,43)
	var y := 149
	for bus in ["Music","SFX"]:
		_label("Music" if bus == "Music" else "Sound effects",44,y,21)
		var slider := HSlider.new()
		slider.position = Vector2(44,y+47)
		slider.custom_minimum_size = Vector2(430,31)
		slider.step = 5
		slider.value = sound.level(bus)
		page.add_child(slider)
		slider.value_changed.connect(func(v): sound.set_level(bus,v))
		if bus == "Music":
			slider.grab_focus()
		y += 114
	_button("Back",44,439,180,_main_menu)

func _unhandled_key_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		match screen:
			"pause": _resume()
			"select", "settings", "result": _main_menu()
			"vs": _selection()
			"controls": _controls_back()
			_: return
		get_viewport().set_input_as_handled()

func stop_audio() -> void:
	if sound:
		sound.player.stop()
	if is_instance_valid(arena):
		arena.set_suspended(true)
		arena.audio.stop()
	# Allow the audio mixer to retire queued playback before Godot destroys the scene.
	await get_tree().create_timer(0.25, true, false, true).timeout

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST and is_inside_tree():
		await stop_audio()
		get_tree().quit()
