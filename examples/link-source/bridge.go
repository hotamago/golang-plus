package main

func callFromGoFile() string {
	return "[same package go] " + fromGpFile()
}
