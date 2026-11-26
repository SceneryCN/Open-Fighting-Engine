# SPDX-License-Identifier: MPL-2.0
extends Node

const SETTINGS := "user://demo-settings.cfg"
var player := AudioStreamPlayer.new()
var preferences := ConfigFile.new()

func _ready() -> void:
	preferences.load(SETTINGS)
	for bus in ["Music", "SFX"]:
		if AudioServer.get_bus_index(bus) < 0:
			AudioServer.add_bus()
			AudioServer.set_bus_name(AudioServer.bus_count - 1, bus)
		_apply(bus, level(bus))
	player.stream = preload("res://demo/viaduct.wav")
	player.bus = &"Music"
	add_child(player)
	player.play()

func level(bus: String) -> float:
	return clampf(float(preferences.get_value("audio", bus, 65.0)), 0, 100)

func set_level(bus: String, amount: float) -> void:
	_apply(bus, amount)
	preferences.set_value("audio", bus, amount)
	if preferences.save(SETTINGS) != OK:
		push_warning("Could not save audio preferences.")

func _apply(bus: String, amount: float) -> void:
	var index := AudioServer.get_bus_index(bus)
	AudioServer.set_bus_mute(index, amount <= 0)
	AudioServer.set_bus_volume_db(index, linear_to_db(maxf(amount / 100.0, 0.0001)))

func _exit_tree() -> void:
	player.stop()
	player.stream = null
