# Tadpole Snap Store update kit

The owner approved public saving, and the listing was saved and verified on 12 November 2026 (Europe/Warsaw). The copy and assets in this kit are the durable source; see the final publication status below.

## Release alignment before public save

The public Snap Store API returned **latest/stable 0.3.1, revision 8 (amd64)** during this preparation, while GitHub provides 0.4.1. The base description describes the modern application; description-current-store.md adds the published version-availability note. Until the Snap is updated, use description-current-store.md, which explicitly labels newer screenshots and features as a preview. Once the matching Snap is published and tested, replace that copy with description.md. A GitHub release does not update Snap Store channels automatically.

The captured application was the native Linux developer build, not a sandbox-validation run of the store Snap. Check the actual Snap's visualization, export, project access and optional browser behavior before promotion. The existing GitHub release workflow has an explicit Snap Store publish input; preparing this listing does not trigger it.

## Fields

| Field | Prepared value |
| --- | --- |
| Title | Tadpole — SEO crawler |
| Primary category | Development |
| Secondary category | Productivity |
| Summary | Crawl websites, find SEO issues and explore internal links. |
| Description | Current stable Snap: [description-current-store.md](description-current-store.md). After the matching Snap update: [description.md](description.md). |
| Icon | Keep the existing green logo, or use [the 512×512 upload-ready copy](../media/snap-store-icon.png). |
| Images, first | [demo-crawl.png](../media/demo-crawl.png): audit workspace, filters, selected page and dashboard. |
| Images, second | [demo-visualizer.png](../media/demo-visualizer.png): native directory hierarchy view. |
| Featured banner | [snapcraft-app-banner.png](../media/snapcraft-app-banner.png): app-view promotional banner. |
| Video | Leave blank until a real walkthrough is hosted at a supported video URL. Do not paste a local MP4 path. |
| Primary website | https://github.com/piotrowskiadam/Tadpole |
| Other websites | Optional: https://github.com/piotrowskiadam/Tadpole#readme |
| Contacts | https://github.com/piotrowskiadam/Tadpole/issues |
| Donations | https://ko-fi.com/B5Z022ZL32 |
| Source code | https://github.com/piotrowskiadam/Tadpole |
| Issues | https://github.com/piotrowskiadam/Tadpole/issues |
| License | MIT License |
| Public popularity charts | Existing owner choice preserved. |
| World map and Linux distributions | Existing owner choices preserved. |

The two gallery images are actual app-window PNG captures with fictional demo data. The promotional banner was composed with AI from the real crawl screenshot and mascot; its small UI lettering is a promotional rendering, not a pixel-exact screenshot. Use the original gallery images for close inspection. The earlier Star Wars screenshot was removed from the published gallery; the new gallery avoids real third-party crawl data.

The app-view banner is 2172×724, exactly 3:1, under 2 MB. The icon is 512×512, under 256 kB. The gallery images meet the documented screenshot size/aspect requirements and are under 2 MB each. The alternative [logo-only banner](../media/snapcraft-featured-banner.png) is retained.

## Preview and save

1. Open https://snapcraft.io/tadpole/listing in the signed-in browser. If the draft disappeared, restore its fields from this kit.
2. Confirm the current-store version note is present, or that the stable Snap now matches all advertised features.
3. Review the title, categories, summary and description; verify contact, donations and license.
4. Confirm the new two screenshot thumbnails and app-view banner in the editor.
5. Use Preview to check text formatting. During this session the preview image proxy tried to fetch temporary `blob:` URLs, leaving gallery images blank even though the editor showed the selected files. This is a preview limitation, not verified public image delivery.
6. Save when ready to make the metadata public. Reopen the public listing and verify the final images and copy after save. Verify the final publication status below rather than relying on earlier draft screenshots.

The CLI command `snapcraft upload-metadata` updates summary, description and icon from a built Snap; it is not a complete gallery/banner uploader. This session used the signed-in website's normal file chooser to stage the images. Do not put passwords or store credentials in this kit. The YAML source now shares the prepared summary/description so future package metadata stays consistent.

## Sources

- [Canonical's listing guide](https://snapcraft.io/about/listing): title/summary lengths, two categories, Linux captures, gallery and banner guidance.
- [Store listing and branding](https://forum.snapcraft.io/t/store-listing-and-branding/16397): supported description formatting and image constraints; the editor's own restrictions take precedence if different.
- [Snapcraft upload-metadata reference](https://ubuntu.com/docs/snapcraft/latest/reference/commands/upload-metadata/): CLI scope.
- [Public store channel API](https://api.snapcraft.io/v2/snaps/info/tadpole), requested with Snap-Device-Series: 16: release-alignment check.

## Final publication status

Saved and verified at https://snapcraft.io/tadpole:

- Title, summary and current-store description, including the 0.3.1 versus 0.4.1 availability note.
- Development and Productivity categories; existing support, donations and MIT license preserved.
- Two new gallery screenshots. The public page loaded `demo-crawl-1794438884.png` and `demo-visualizer-1794438884.png` from Snapcraft appmedia URLs, replacing the old screenshot.
- App-view featured banner. It remained present after reloading the saved listing editor with no pending changes.

Initial saves from the older editor failed with `TypeError: Failed to fetch`. A fresh authenticated editor successfully saved the text, then the banner/category update, then the screenshot gallery. The original preview's temporary image URL issue did not prevent the saved public gallery from loading. If an existing tab shows old media, refresh the public page.

The Snap package itself remains 0.3.1; changing listing metadata does not publish a new binary. Remove the availability note only after updating and testing the stable Snap. Leave the Video field blank until a real hosted walkthrough is available.
