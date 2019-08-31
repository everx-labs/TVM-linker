for filename in contract*.sol; do
    basename="$(echo $filename | sed 's/\.sol//')"
    echo "Compiling $filename..."
    bash ./compile2.sh $basename

done
