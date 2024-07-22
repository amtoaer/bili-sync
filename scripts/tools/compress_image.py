"""
供开发者使用的图片压缩工具，批量将项目中的图片压缩为 webp 格式
"""

import os


def main():
    for root, dirs, files in os.walk(".", topdown=True):
        dirs[:] = [d for d in dirs if d != "dist" and not d.startswith(".")]
        if all(dir_name not in root for dir_name in ("assets", "static", "public")):
            continue
        for file in files:
            if "icon" in file or not file.endswith(("jpg", "jpeg", "png")):
                continue
            source, target = file, file[: file.rfind(".")] + ".webp"
            escaped_source, escaped_target = (
                source.replace(".", r"\."),
                target.replace(".", r"\."),
            )
            source_path, target_path = (
                os.path.join(root, source),
                os.path.join(root, target),
            )
            os.system(
                rf"""
                cwebp -q 80 -sharp_yuv -mt -metadata all {source_path} -o {target_path} && \
                rm {source_path} && \
                rg {source} --files-with-matches --no-messages | xargs sed -i '' 's/{escaped_source}/{escaped_target}/g'
                """
            )


if __name__ == "__main__":
    main()
