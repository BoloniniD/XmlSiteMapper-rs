# XmlSiteMapper-rs
A simple sitemapper written in Rust. Crawles through the site and creates xml sitemap.

# How to install
Go to Releases section and download an archive with executable for your system. Currently releases contain executables for Windows and Ubuntu-like Linux systems. After downloading the release, unpack the archive in any folder and launch the executable. It is recommended to launch the sitemapper from terminal or cmd.

# How to use
After first launch described in previous paragraph, sitemapper will generate its configuration files: site.cfg, change_prio.cfg and disallow.cfg. It is required to fill in site.cfg, since it contains your site's root URL and delay (optional parameter, set to 25ms by default) between requests, which is needed if your site blocks too frequent requests. After that, you may want to provide additional configs in change_prio.cfg (to change <priority> field for special URL queries) and disallow.cfg (to exclude URLs, which lead to files with listed extensions). After that you can launch the program again and wait for sitemap.xml to be generated.

You can also provide a path to desired sitemap.xml location. For example, if you have XmlSiteMapper-rs in folder "/cool_folder/site/mapper/" and you want to generate sitemap with path "/cool_folder/map/sitemap.xml", you will need to provide an absolute path to that directory, which is "/cool_folder/map/". Note, that sitemapper may be unable to create a file in some folders due to lack of permissions, so you will need to run it as admin/sudo.

# P.S.
The work on this sitemapper started not so long ago, it is planned to add more functionality and more user-friendly UI in nearest future.

