#!/bin/sh
# 从原图出全部图标（施工 DT-5）。原图 logo.png 是项目主人画的黑白剪影（1254px，黑色圆形里一个白色的少女，蝴蝶结伸出圆外）。
# 换图：换掉 logo.png，跑一遍这个脚本，把 icons/ 下的结果一起提交。要 potrace、rsvg-convert、ImageMagick 7、oxipng。
#
# 出四套：
# - 应用图标：浅色圆角底板上的整张徽章。48px 以上用原样描的 miyu.svg，32px 以下用去掉细发丝的 miyu-simple.svg（小了细线糊成灰）。
# - 托盘（Linux、Windows）：小尺寸的应用图标，带底板，深色、浅色的任务栏上都看得见。
# - 托盘（macOS）：模板图，黑色加透明，系统自己照菜单栏着色。
# - 菜单里的图标（施工 AP-2）：lucide/ 下的 Lucide 原稿（ISC 许可，见 lucide/LICENSE；和网页用的同一套），灰色线条 32px。
#   深色、浅色菜单上都看得见；macOS 当模板图用，系统照菜单着色。只出这一套：sh build.sh menu。
set -eu
cd "$(dirname "$0")"
out=..
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# 菜单里的图标：线条换成灰色，照 32px 出。
menu() {
    mkdir -p $out/menu
    for name in square play rotate-cw power; do
        sed 's/currentColor/#808080/' "lucide/$name.svg" > "$tmp/$name.svg"
        rsvg-convert -w 32 -h 32 "$tmp/$name.svg" -o "$out/menu/$name.png"
    done
    oxipng -q -o 4 --strip safe --nc $out/menu/*.png
}
if [ "${1:-}" = menu ]; then menu; exit 0; fi

# 底板的颜色和圆角；徽章放在底板正中，四周留白。
plate='#F6F2EA'
radius=0.22

# 描：原样的一份；先糊再描的一份，细发丝没了，大的形状留着。
magick logo.png -colorspace gray -threshold 50% "$tmp/full.pbm"
potrace "$tmp/full.pbm" -s -o "$tmp/full.svg" --turdsize 8 --alphamax 1.0 --opttolerance 0.2
magick logo.png -colorspace gray -blur 0x6 -threshold 50% "$tmp/simple.pbm"
potrace "$tmp/simple.pbm" -s -o "$tmp/simple.svg" --turdsize 400 --alphamax 1.2 --opttolerance 0.5

# 徽章（黑色部分）的外框，照它算正中和留白。
box=$(magick logo.png -colorspace gray -threshold 50% -negate -format '%@' info:)
w=${box%%x*}; rest=${box#*x}; h=${rest%%+*}; rest=${rest#*+}; x=${rest%%+*}; y=${rest#*+}
side=$(( (w > h ? w : h) * 122 / 100 ))
vx=$(( x + w / 2 - side / 2 )); vy=$(( y + h / 2 - side / 2 ))

# 一份描出来的 SVG 换成：只要路径那一段（potrace 的 <g>），套上自己的 viewBox，要底板的加底板。
wrap() { # 描出来的 svg、要不要底板（plate / none）、输出
    paths=$(sed -n '/<g transform/,/<\/g>/p' "$1")
    if [ "$2" = plate ]; then
        bg="<rect x=\"$vx\" y=\"$vy\" width=\"$side\" height=\"$side\" rx=\"$(awk "BEGIN{print $side*$radius}")\" fill=\"$plate\"/>"
    else
        bg=""
    fi
    printf '<svg xmlns="http://www.w3.org/2000/svg" viewBox="%s %s %s %s">%s%s</svg>\n' "$vx" "$vy" "$side" "$side" "$bg" "$paths" > "$3"
}
wrap "$tmp/full.svg" none miyu.svg
wrap "$tmp/simple.svg" none miyu-simple.svg
wrap "$tmp/full.svg" plate "$tmp/app.svg"
wrap "$tmp/simple.svg" plate "$tmp/app-simple.svg"

png() { # svg、边长、输出
    rsvg-convert -w "$2" -h "$2" "$1" -o "$3"
}
for s in 16 24 32; do png "$tmp/app-simple.svg" $s "$tmp/$s.png"; done
for s in 48 64 128 256 512; do png "$tmp/app.svg" $s "$tmp/$s.png"; done

# Tauri 照 tauri.conf.json 的 bundle.icon 找这些（Windows 编进程序的是 icon.ico；macOS 的 .icns 打包时从 png 出）。
cp "$tmp/512.png" $out/icon.png
cp "$tmp/32.png" $out/32x32.png
cp "$tmp/128.png" $out/128x128.png
cp "$tmp/256.png" $out/256x256.png
magick "$tmp/16.png" "$tmp/24.png" "$tmp/32.png" "$tmp/48.png" "$tmp/64.png" "$tmp/256.png" $out/icon.ico

# 托盘：Linux、Windows 带底板的 32px（高分屏缩着用 64px 的），macOS 模板图 44px（22pt 的两倍）。
png "$tmp/app-simple.svg" 64 $out/tray.png
png miyu-simple.svg 44 $out/tray-template.png

# Tauri 只认 RGBA 的 png：压的时候不许改颜色类型。
oxipng -q -o 4 --strip safe --nc $out/*.png
menu
