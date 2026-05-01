package main

type ResultTag int

const (
	ResultTagOk ResultTag = iota
	ResultTagErr
)

type Result[T any] struct {
	tag  ResultTag
	ok0  T
	err0 string
}

func ResultOk[T any](v0 T) Result[T] {
	return Result[T]{tag: ResultTagOk, ok0: v0}
}

func ResultErr[T any](v0 string) Result[T] {
	return Result[T]{tag: ResultTagErr, err0: v0}
}

func load() Result[int] {
	return ResultOk[int](7)
}
