package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/grantyap/qonvert/internal/transcode"
	"github.com/spf13/cobra"
	"github.com/vbauerster/mpb/v8"
	"github.com/vbauerster/mpb/v8/decor"
)

var (
	outputPath string
	outputType string
	codec      string
	limit      uint

	rootCommand = &cobra.Command{
		Use:   "qo",
		Short: "A tiny CLI for batch video conversion",
		Args:  cobra.MinimumNArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			start := time.Now()

			inputFilePaths, err := transcode.FilePathFromArgs(args)
			if err != nil {
				cmd.PrintErrln("could not extract file paths from args", err)
			}

			filePaths, err := transcode.NewItems(outputPath, inputFilePaths, outputType)
			if err != nil {
				cmd.PrintErrln(err)
			}

			items := transcode.ReadFrameCounts(filePaths, 50)

			cmd.Printf("transcoding %v items\n", len(items))

			var wg sync.WaitGroup
			progress := mpb.New(mpb.WithWaitGroup(&wg))

			type ItemWithProgressBar struct {
				Item        *transcode.ItemWithProgress
				ProgressBar *mpb.Bar
			}

			wg.Add(len(items))
			jobs := make(chan ItemWithProgressBar, len(items))

			for i := uint(0); i < limit; i++ {
				go func() {
					for item := range jobs {
						progress := make(chan transcode.ItemWithProgress)
						previousFrameCount := item.Item.CurrentFrame
						previousTime := time.Now()

						go func() {
							for p := range progress {
								deltaFrames := p.CurrentFrame - previousFrameCount
								now := time.Now()
								deltaTime := now.Sub(previousTime)
								previousTime = now

								previousFrameCount = p.CurrentFrame
								item.ProgressBar.EwmaIncrBy(int(deltaFrames), deltaTime)
							}
						}()

						err := transcode.Execute(*item.Item, codec, progress)
						if err != nil {
							cmd.Println("failed:", item.Item.Item.OutputPath, err)
						}

						wg.Done()
					}
				}()
			}

			for _, item := range items {
				name, err := filepath.Rel(outputPath, item.Item.OutputPath)
				if err != nil {
					cmd.PrintErr(err)
					continue
				}

				bar := progress.AddBar(int64(item.FrameCount),
					mpb.PrependDecorators(
						decor.Name(name, decor.WCSyncSpace),
						decor.Any(func(s decor.Statistics) string {
							return fmt.Sprintf("%v/%v", s.Current, s.Total)
						}, decor.WCSyncSpace),
						decor.Percentage(decor.WCSyncSpace),
					),
					mpb.AppendDecorators(
						decor.OnComplete(
							decor.EwmaETA(decor.ET_STYLE_GO, 30, decor.WCSyncWidth), "done",
						),
					),
				)

				jobs <- ItemWithProgressBar{
					Item:        &item,
					ProgressBar: bar,
				}
			}
			close(jobs)

			wg.Wait()

			cmd.Println("successfully transcoded", len(items), "items in", time.Since(start).Round(time.Millisecond))
		},
	}
)

func init() {
	workingDirectory, err := os.Getwd()
	if err != nil {
		panic(err)
	}

	rootCommand.PersistentFlags().StringVarP(&outputPath, "output-path", "o", workingDirectory, "file path containing all the transcoded output videos")
	rootCommand.PersistentFlags().StringVarP(&outputType, "output-type", "t", "", "output file extension")
	rootCommand.PersistentFlags().StringVarP(&codec, "codec", "c", "", "video codec to use for transcoding")
	rootCommand.PersistentFlags().UintVarP(&limit, "limit", "l", 5, "number of concurrent FFmpeg processes")
}

func Execute() error {
	return rootCommand.Execute()
}
