# SPDX-License-Identifier: MPL-2.0
extends Control

var bars: Array[ProgressBar] = []
var names: Array[Label] = []
var values: Array[Label] = []
var energy: Array[Label] = []
var clock_label: Label
var message: Label

func _panel(x: int, y: int, w: int, h: int, tint: Color) -> void:
	var panel := ColorRect.new()
	panel.position = Vector2(x, y)
	panel.size = Vector2(w, h)
	panel.color = tint
	panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(panel)

func _label(x: int, y: int, w: int, h: int, pixels: int, alignment: HorizontalAlignment, tint: Color, caption := "") -> Label:
	var node := Label.new()
	node.position = Vector2(x, y)
	node.size = Vector2(w, h)
	node.horizontal_alignment = alignment
	node.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	node.add_theme_font_size_override("font_size", pixels)
	node.add_theme_color_override("font_color", tint)
	node.text = caption
	add_child(node)
	return node

func _ready() -> void:
	var ink := Color("#192a31")
	var paper := Color("#fff0cf")
	_panel(0, 0, 960, 86, ink)
	_panel(0, 484, 960, 56, ink)
	for i in range(2):
		var x := 26 if i == 0 else 572
		var accent := Color("#ffac60") if i == 0 else Color("#9bcbd6")
		var align := HORIZONTAL_ALIGNMENT_LEFT if i == 0 else HORIZONTAL_ALIGNMENT_RIGHT
		names.append(_label(x, 7, 362, 29, 21, align, paper))
		var bar := ProgressBar.new()
		bar.position = Vector2(x, 41)
		bar.show_percentage = false
		bar.fill_mode = ProgressBar.FILL_BEGIN_TO_END if i == 0 else ProgressBar.FILL_END_TO_BEGIN
		for style in ["background", "fill"]:
			var box := StyleBoxFlat.new()
			box.bg_color = Color("#435158") if style == "background" else accent
			bar.add_theme_stylebox_override(style, box)
		add_child(bar)
		bar.size = Vector2(362, 9)
		bars.append(bar)
		values.append(_label(x, 53, 362, 24, 14, align, accent))
		energy.append(_label(x, 53, 362, 24, 13, HORIZONTAL_ALIGNMENT_RIGHT if i == 0 else HORIZONTAL_ALIGNMENT_LEFT, paper))
	_label(402, 8, 156, 24, 13, HORIZONTAL_ALIGNMENT_CENTER, Color("#c8c7ac"), "MATCH TIME")
	clock_label = _label(402, 28, 156, 41, 30, HORIZONTAL_ALIGNMENT_CENTER, paper)
	_label(24, 487, 910, 23, 14, HORIZONTAL_ALIGNMENT_LEFT, paper, "P1    A / D  Move     W  Jump     S  Lane     J or O  Attack     K  Guard     L  Dash")
	_label(24, 510, 910, 24, 14, HORIZONTAL_ALIGNMENT_LEFT, paper, "P2    ← / →  Move     ↑  Jump     ↓  Lane     F  Attack     G  Guard     H  Dash")
	_label(776, 494, 158, 32, 14, HORIZONTAL_ALIGNMENT_RIGHT, Color("#eab77e"), "R   Restart")
	message = _label(40, 119, 880, 48, 23, HORIZONTAL_ALIGNMENT_CENTER, ink)

func present(snap: Dictionary) -> void:
	for i in range(2):
		var f: Dictionary = snap.fighters[i]
		names[i].text = ("P%d   /   %s" % [i + 1, str(f.name).to_upper()]) if i == 0 else ("%s   /   P2" % str(f.name).to_upper())
		bars[i].max_value = f.max_hp
		bars[i].value = f.hp
		values[i].text = "%d / %d  HP" % [f.hp, f.max_hp]
		energy[i].text = "%s   ·   EN  %d" % [str(f.state).to_upper(), f.energy]
	var seconds := floori(float(snap.frame) / 60.0)
	clock_label.text = "%02d:%02d" % [floori(seconds / 60.0), seconds % 60]
