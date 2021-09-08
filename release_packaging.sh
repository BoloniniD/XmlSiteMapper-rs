cd target/release/
if [ -f Linux.x86_64.tar ]; then
    rm Linux.x86_64.tar
fi
echo "Creating .tar archive for Linux"
tar -cvf Linux.x86_64.tar XmlSiteMapper-rs
if [ -f Windows.x86_64.zip ]; then
    rm Windows.x86_64.zip
fi
echo "Creating .zip archive for Windows"
zip -0 Windows.x86_64.zip XmlSiteMapper-rs.exe
echo "Job finished"