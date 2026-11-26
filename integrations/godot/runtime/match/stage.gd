# SPDX-License-Identifier: MPL-2.0
extends Node2D

# The stage is static. Godot retains these drawing commands between frames.
func _draw() -> void:
	draw_rect(Rect2(0, 0, 960, 540), Color("#eb9e77"))
	draw_circle(Vector2(682, 205), 60, Color("#ffe8b1"))
	_ridge([0, 280, 100, 205, 150, 235, 270, 146, 365, 230, 434, 192, 528, 268, 645, 158, 723, 222, 828, 134, 960, 212], Color("#bb7973"))
	_ridge([0, 294, 126, 245, 201, 261, 302, 211, 398, 253, 522, 219, 591, 243, 715, 192, 832, 246, 960, 195], Color("#87636b"))
	_ridge([0, 299, 152, 278, 272, 288, 409, 251, 530, 285, 652, 246, 762, 262, 891, 231, 960, 245], Color("#535860"))
	for x in range(-42, 1020, 184):
		draw_rect(Rect2(x, 0, 13, 325), Color("#343e43"))
		draw_rect(Rect2(x + 13, 0, 3, 325), Color("#986e5c"))
		draw_line(Vector2(x + 10, 129), Vector2(x + 107, 0), Color("#41484a"), 8)
	draw_rect(Rect2(0, 115, 960, 7), Color("#515456"))
	draw_rect(Rect2(0, 300, 960, 8), Color("#303e43"))
	for x in range(0, 980, 38):
		draw_rect(Rect2(x, 306, 3, 39), Color("#435054"))
	draw_rect(Rect2(0, 335, 960, 205), Color("#425b61"))
	for y in [341, 351, 368, 394, 435, 498]:
		draw_line(Vector2(0, y), Vector2(960, y), Color("#364e55"), 2)
	for x in range(-1700, 2700, 140):
		draw_line(Vector2(480 + (x - 480) * 0.12, 335), Vector2(x, 540), Color("#354e55"), 2)
	for y in [367, 421]:
		draw_line(Vector2(0, y), Vector2(960, y), Color("#9aa49a"), 2)
	for x in range(0, 960, 36):
		draw_line(Vector2(x, 422), Vector2(x + 12, 422), Color("#dcc09a"), 2)

func _ridge(coords: Array, color: Color) -> void:
	var points := PackedVector2Array()
	for i in range(0, coords.size(), 2):
		points.append(Vector2(coords[i], coords[i + 1]))
	points.append(Vector2(960, 340))
	points.append(Vector2(0, 340))
	draw_colored_polygon(points, color)
