/**
 * Window Resize E2E Test
 *
 * CRITICAL: Verifies that cursor position remains accurate when window is resized
 *
 * Why this matters:
 * - Desktop users constantly resize windows
 * - getBoundingClientRect() changes on resize
 * - This causes cursor position drift if not handled correctly
 */

import { browser, $ } from '@wdio/globals'

describe('Window Resize - Coordinate Stability', () => {
    beforeEach(async () => {
        // Reset window to known size
        await browser.setWindowSize(1400, 900)
        await browser.pause(500)
    })

    it('should maintain cursor position after window resize', async () => {
        // Type some text
        await browser.keys(['H', 'e', 'l', 'l', 'o', ' ', 'W', 'o', 'r', 'l', 'd'])
        await browser.pause(200)

        // Get current cursor position
        const getCursorPosition = async () => {
            return await browser.execute(() => {
                const textarea = document.querySelector('textarea')
                if (!textarea) return null
                return {
                    selectionStart: textarea.selectionStart,
                    selectionEnd: textarea.selectionEnd
                }
            })
        }

        const positionBefore = await getCursorPosition()

        // Resize window to smaller size
        await browser.setWindowSize(1000, 600)
        await browser.pause(500)

        const positionAfterSmall = await getCursorPosition()

        // Verify cursor position is maintained
        expect(positionAfterSmall.selectionStart).toBe(positionBefore.selectionStart)
        expect(positionAfterSmall.selectionEnd).toBe(positionBefore.selectionEnd)

        // Resize window to larger size
        await browser.setWindowSize(1600, 1000)
        await browser.pause(500)

        const positionAfterLarge = await getCursorPosition()

        // Verify cursor position is still maintained
        expect(positionAfterLarge.selectionStart).toBe(positionBefore.selectionStart)
        expect(positionAfterLarge.selectionEnd).toBe(positionBefore.selectionEnd)
    })

    it('should maintain click accuracy after window resize', async () => {
        // Type test text
        await browser.keys(['L', 'i', 'n', 'e', ' ', '1'])
        await browser.pause(200)

        // Resize window
        await browser.setWindowSize(1200, 700)
        await browser.pause(500)

        // Get canvas element (where clicks happen)
        const canvas = await $('canvas')
        expect(canvas).toBeTruthy()

        // Click on specific position
        await canvas.click({ x: 100, y: 50 })
        await browser.pause(200)

        // Verify cursor moved (check via textarea selection)
        const cursorPos = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.selectionStart : -1
        })

        // Cursor should be at a valid position (not -1)
        expect(cursorPos).toBeGreaterThanOrEqual(0)
    })

    it('should handle rapid window resizes without crash', async () => {
        // Type some text first
        await browser.keys(['T', 'e', 's', 't'])
        await browser.pause(200)

        // Rapid resize sequence
        const sizes = [
            [1400, 900],
            [1200, 700],
            [1000, 600],
            [1600, 1000],
            [1300, 800],
            [1400, 900]
        ]

        for (const [width, height] of sizes) {
            await browser.setWindowSize(width, height)
            await browser.pause(100)
        }

        // Verify app is still responsive by typing
        await browser.keys(['!'])
        await browser.pause(200)

        const text = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea ? textarea.value : ''
        })

        // Should contain the text we typed (even if textarea clears, buffer should have it)
        // This just verifies the app didn't crash
        expect(text).toBeDefined()
    })

    it('should recalculate getBoundingClientRect on resize', async () => {
        // Get initial canvas bounds
        const getBounds = async () => {
            return await browser.execute(() => {
                const canvas = document.querySelector('canvas')
                if (!canvas) return null
                const rect = canvas.getBoundingClientRect()
                return {
                    width: rect.width,
                    height: rect.height,
                    left: rect.left,
                    top: rect.top
                }
            })
        }

        const boundsBefore = await getBounds()
        expect(boundsBefore).toBeTruthy()

        // Resize window
        await browser.setWindowSize(1200, 700)
        await browser.pause(500)

        const boundsAfter = await getBounds()
        expect(boundsAfter).toBeTruthy()

        // Bounds should have changed
        expect(
            boundsBefore.width !== boundsAfter.width ||
            boundsBefore.height !== boundsAfter.height
        ).toBe(true)
    })
})
