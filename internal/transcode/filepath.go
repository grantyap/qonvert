package transcode

import (
	"os"
	"path/filepath"
)

type FilePath = string
type DirPath = string

func FilePathFromArgs(args []string) ([]FilePath, error) {
	filePaths := make([]FilePath, 0, len(args))
	for _, arg := range args {
		path, err := filepath.Abs(arg)
		if err != nil {
			return nil, err
		}
		filePaths = append(filePaths, path)
	}

	if len(filePaths) == 1 {
		fileInfo, err := os.Stat(filePaths[0])
		if err != nil {
			return nil, err
		}

		if fileInfo.IsDir() {
			return fromDir(filePaths[0])
		}
	}

	return filePaths, nil
}

func NewItems(basePath DirPath, filePaths []FilePath, outputFileType string) ([]Item, error) {
	newPaths := make([]Item, 0, len(filePaths))
	for _, filePath := range filePaths {
		newPath := filepath.Join(basePath, withExt(filepath.Base(filePath), outputFileType))
		newPaths = append(newPaths, Item{
			InputPath:  filePath,
			OutputPath: newPath,
		})
	}

	return newPaths, nil
}

func fromDir(path DirPath) ([]FilePath, error) {
	entries, err := os.ReadDir(path)
	if err != nil {
		return nil, err
	}

	paths := make([]FilePath, 0, len(entries))
	for _, entry := range entries {
		if !entry.IsDir() {
			paths = append(paths, filepath.Join(path, entry.Name()))
		}
	}

	return paths, nil
}

func withExt(path FilePath, ext string) string {
	oldExtension := filepath.Ext(path)
	return path[0:len(path)-len(oldExtension)] + "." + ext
}
