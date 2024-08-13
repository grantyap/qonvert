package transcode

import (
	"fmt"
	"log"
	"math/rand"
	"net"
	"os"
	"os/exec"
	"path"
	"regexp"
	"strconv"
	"strings"
)

var defaultCodecArgs = map[string][]string{
	"libx265": {
		// Support h.265 thumbnail previews on Apple devices.
		"-tag:v", "hvc1",

		// The default of 28 has clearly worse quality. 24 looks good enough with significant size improvements.
		"-crf", "24",
	},
	"hevc_videotoolbox": {
		// Support h.265 thumbnail previews on Apple devices.
		"-tag:v", "hvc1",

		// 65 quality is a good quality/size ratio.
		// Any higher and the size starts to explode. Any lower and the quality starts to look significantly worse.
		"-q:v", "65",
	},
}

func buildArgs(item Item, codec string, socketFilePath string) []string {
	args := []string{
		// Emit progress to the socket file.
		"-progress", "unix://" + socketFilePath,

		// Overwrite the output file.
		// TODO: Maybe provide an option for overriding existing files?
		"-y",

		// The input file.
		"-i", item.InputPath,
	}

	if codec != "" {
		args = append(args,
			"-c:v", codec,
		)
		args = append(args,
			defaultCodecArgs[codec]...,
		)
	}

	args = append(args,
		// Allows h.264 and h.265 to start streaming earlier.
		"-movflags", "faststart",

		// Ensure that `.gif` colors are correctly converted.
		"-pix_fmt", "yuv420p",

		// Ensure that the dimensions are divisible by 2.
		"-vf", "crop=trunc(iw/2)*2:trunc(ih/2)*2",
	)

	args = append(args, item.OutputPath)

	return args
}

type Result struct {
	Item  ItemWithProgress
	Error error
}

func Execute(item ItemWithProgress, codec string) <-chan Result {
	sockFilePath, progress := readProgress(item)
	result := make(chan Result)
	go func() {
		for p := range progress {
			result <- Result{Item: p, Error: nil}
		}
	}()

	args := buildArgs(*item.Item, codec, sockFilePath)
	cmd := exec.Command("ffmpeg", args[:]...)
	go func() {
		err := cmd.Run()
		result <- Result{Item: item, Error: err}
		close(result)
	}()

	return result
}

// Taken from https://github.com/u2takey/ffmpeg-go/blob/898ebfd93985f0f69cde36e466094cd453caa349/examples/showProgress.go#L41
func readProgress(item ItemWithProgress) (string, <-chan ItemWithProgress) {
	progress := make(chan ItemWithProgress)

	socketFilePath := path.Join(os.TempDir(), fmt.Sprintf("%d_sock", rand.Int()))
	l, err := net.Listen("unix", socketFilePath)
	if err != nil {
		panic(err)
	}

	go func() {
		re := regexp.MustCompile(`frame=(\d+)`)

		fd, err := l.Accept()
		if err != nil {
			log.Fatal("accept error:", err)
		}

		buf := make([]byte, 16)
		data := ""

		for {
			_, err := fd.Read(buf)
			if err != nil {
				fmt.Println(err)
				close(progress)
				return
			}

			data += string(buf)
			a := re.FindAllStringSubmatch(data, -1)
			value := uint64(0)

			if len(a) > 0 && len(a[len(a)-1]) > 0 {
				c, err := strconv.ParseUint(a[len(a)-1][len(a[len(a)-1])-1], 10, 64)
				if err != nil {
					fmt.Println(err)
					close(progress)
					return
				}
				value = c
			}

			if strings.Contains(data, "progress=end") {
				progress <- ItemWithProgress{Item: item.Item, CurrentFrame: item.FrameCount, FrameCount: item.FrameCount}
				close(progress)
				return
			}

			progress <- ItemWithProgress{Item: item.Item, CurrentFrame: value, FrameCount: item.FrameCount}
		}
	}()

	return socketFilePath, progress
}
