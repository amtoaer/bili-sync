"""
2.0.2 -> 2.0.3 时添加了将 poster 拷贝为 fanart 的行为
该行为对已存在的视频不会生效，所以可以手动执行该脚本
具体来说，该脚本会：
1. 遍历命令行参数中所有存在的路径
2. 找到路径中所有以 poster.jpg 结尾的文件
3. 将 poster.jpg 替换为 fanart.jpg，拷贝到同一目录
"""

import os
import sys
import shutil
from pathlib import Path


def main():
    if len(sys.argv) <= 1:
        print("用法： python 2.0.3_add_fanart.py <path1> <path2> ...")
        exit(1)
    paths = [Path(path) for path in sys.argv[1:]]
    for path in paths:
        if not path.exists():
            print(f"路径 {path} 不存在，跳过..")
            continue
        for root, _, files in os.walk(path):
            for file in files:
                if file.endswith("poster.jpg"):
                    poster_path = Path(root) / file
                    print(f"已找到 poster: {poster_path}")
                    fanart_path = Path(root) / file.replace("poster.jpg", "fanart.jpg")
                    shutil.copyfile(poster_path, fanart_path)
                    print(f"已将 {poster_path} 拷贝至 {fanart_path}")
    print("操作完成")


if __name__ == "__main__":
    main()
