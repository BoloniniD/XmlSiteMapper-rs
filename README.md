# BoloniniMapper
A simple sitemapper written in Rust. Crawles through the site and creates xml sitemap.

# How to use
Place the executable in some directory, then launch it. Mapper will create .cfg files for configuration. Fill in the site.cfg file, and provide additional configs in change_prio.cfg and disallow.cfg. After that you can launch it again and wait for sitemap.xml to be generated.
