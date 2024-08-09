package transcode

import (
	"bytes"
	"os/exec"
	"strconv"
	"strings"
)

func ReadFrameCount(path FilePath) (uint64, error) {
	durationStr, err := probe(path)
	if err != nil {
		return 0, err
	}

	duration, err := strconv.ParseUint(durationStr, 10, 64)
	if err != nil {
		return 0, err
	}

	return duration, nil
}

func probe(path FilePath) (string, error) {
	var args = [...]string{
		"-v", "error",

		// TODO: Maybe handle audio streams as well?
		"-select_streams", "v:0",

		"-count_packets",
		"-show_entries", "stream=nb_read_packets",
		"-of", "csv=p=0",
		path,
	}

	cmd := exec.Command("ffprobe", args[:]...)
	var buffer bytes.Buffer
	cmd.Stdout = &buffer

	err := cmd.Run()
	if err != nil {
		return "", err
	}

	return strings.TrimSpace(buffer.String()), nil
}

type Item struct {
	InputPath  string
	OutputPath string
}
type ItemWithProgress struct {
	Item         *Item
	CurrentFrame uint64
	FrameCount   uint64
}

func ReadFrameCounts(items []Item, workers int) []ItemWithProgress {
	jobs := make(chan Item, len(items))
	results := make(chan *ItemWithProgress, len(items))

	for w := 1; w <= workers; w++ {
		go readFrameCountsWorker(jobs, results)
	}

	for _, item := range items {
		jobs <- item
	}
	close(jobs)

	itemsWithDuration := make([]ItemWithProgress, 0, len(items))
	for i := 1; i <= len(items); i++ {
		itemWithDuration := <-results
		if itemWithDuration == nil {
			continue
		}
		itemsWithDuration = append(itemsWithDuration, *itemWithDuration)
	}
	close(results)

	return itemsWithDuration
}

func readFrameCountsWorker(jobs chan Item, results chan *ItemWithProgress) {
	for job := range jobs {
		duration, err := ReadFrameCount(job.InputPath)
		if err != nil {
			results <- nil
		}

		results <- &ItemWithProgress{Item: &job, FrameCount: duration}
	}
}
