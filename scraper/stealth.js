const puppeteer = require('puppeteer-extra')

const StealthPlugin = require('puppeteer-extra-plugin-stealth')
puppeteer.use(StealthPlugin())

puppeteer.launch({
    headless: 'shell',
    executablePath: '/home/ubuntu/projects/chrome-headless-shell/linux-126.0.6478.63/chrome-headless-shell-linux64/chrome-headless-shell'
}).then(async browser => {
    console.log('Running tests..')
    const page = await browser.newPage()
    await page.goto('https://bot.sannysoft.com')
    await page.waitForNetworkIdle()
    await page.screenshot({ path: 'testresult.png', fullPage: true })
    await browser.close()
    console.log(`All done, check the screenshot. ✨`)
})


// npx @puppeteer/browsers install chrome-headless-shell@stable
// export PUPPETEER_PRODUCT = chrome
// npm install puppeteer - extra - plugin - stealth
// wget - qO - https://raw.githubusercontent.com/nvm-sh/nvm/v0.38.0/install.sh | bash
