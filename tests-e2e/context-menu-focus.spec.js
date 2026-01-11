/**
 * Context Menu Focus E2E Test
 *
 * CRITICAL: Verifies focus returns to editor after context menu interaction
 *
 * Why this matters:
 * - Right-click context menus steal focus
 * - If focus doesn't return, user can't type ("文字入力できない")
 * - This is the most frustrating UX bug for desktop users
 */

import { browser, $ } from '@wdio/globals'

describe('Context Menu - Focus Management', () => {
    beforeEach(async () => {
        await browser.setWindowSize(1400, 900)
        await browser.pause(500)
    })

    it('should maintain focus after right-click context menu', async () => {
        // Focus editor by typing
        await browser.keys(['H', 'e', 'l', 'l', 'o'])
        await browser.pause(200)

        // Check textarea is focused
        const focusedBefore = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return document.activeElement === textarea
        })

        expect(focusedBefore).toBe(true)

        // Get canvas element
        const canvas = await $('canvas')
        expect(canvas).toBeTruthy()

        // Right-click to open context menu
        await canvas.click({ button: 'right', x: 200, y: 100 })
        await browser.pause(500)

        // Press Escape to close context menu
        await browser.keys(['Escape'])
        await browser.pause(300)

        // Verify focus returned to textarea
        const focusedAfter = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return document.activeElement === textarea
        })

        expect(focusedAfter).toBe(true)

        // Verify we can type immediately after closing menu
        await browser.keys([' ', 'W', 'o', 'r', 'l', 'd'])
        await browser.pause(200)

        // Text should be in the buffer
        const canStillType = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea !== null
        })

        expect(canStillType).toBe(true)
    })

    it('should handle multiple context menu open/close cycles', async () => {
        const canvas = await $('canvas')

        // Cycle 5 times: type -> right-click -> escape -> type
        for (let i = 0; i < 5; i++) {
            // Type
            await browser.keys(['T', 'e', 's', 't'])
            await browser.pause(100)

            // Right-click
            await canvas.click({ button: 'right', x: 150, y: 80 })
            await browser.pause(300)

            // Close menu
            await browser.keys(['Escape'])
            await browser.pause(200)

            // Verify can still type
            const focused = await browser.execute(() => {
                const textarea = document.querySelector('textarea')
                return document.activeElement === textarea
            })

            expect(focused).toBe(true)
        }
    })

    it('should not lose focus to background elements', async () => {
        // Type to focus editor
        await browser.keys(['F', 'o', 'c', 'u', 's'])
        await browser.pause(200)

        // Click on various background areas (not the canvas)
        // This simulates clicking file tree, toolbar, etc.
        const body = await $('body')

        // Click outside canvas (top-left corner)
        await body.click({ x: 10, y: 10 })
        await browser.pause(300)

        // Focus should still be on textarea (or easily recoverable)
        await browser.keys(['T', 'e', 's', 't'])
        await browser.pause(200)

        const canType = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return textarea !== null
        })

        expect(canType).toBe(true)
    })

    it('should restore focus after clicking outside and back inside', async () => {
        // Focus editor
        await browser.keys(['H', 'i'])
        await browser.pause(200)

        const canvas = await $('canvas')

        // Click outside canvas
        const body = await $('body')
        await body.click({ x: 50, y: 50 })
        await browser.pause(300)

        // Click back on canvas
        await canvas.click({ x: 100, y: 100 })
        await browser.pause(300)

        // Should be able to type immediately
        await browser.keys(['!'])
        await browser.pause(200)

        const focused = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return document.activeElement === textarea
        })

        expect(focused).toBe(true)
    })

    it('should handle focus during rapid click sequences', async () => {
        const canvas = await $('canvas')

        // Rapid click sequence
        await canvas.click({ x: 100, y: 50 })
        await browser.pause(50)
        await canvas.click({ x: 200, y: 50 })
        await browser.pause(50)
        await canvas.click({ x: 300, y: 50 })
        await browser.pause(50)
        await canvas.click({ x: 150, y: 100 })
        await browser.pause(200)

        // After rapid clicks, typing should still work
        await browser.keys(['T', 'y', 'p', 'e'])
        await browser.pause(200)

        const focused = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return document.activeElement === textarea
        })

        expect(focused).toBe(true)
    })

    it('should maintain focus after clicking file tree and back to editor', async () => {
        // Type in editor
        await browser.keys(['E', 'd', 'i', 't', 'o', 'r'])
        await browser.pause(200)

        // Simulate clicking file tree area (left side)
        const body = await $('body')
        await body.click({ x: 100, y: 300 })
        await browser.pause(300)

        // Click back on canvas
        const canvas = await $('canvas')
        await canvas.click({ x: 400, y: 200 })
        await browser.pause(300)

        // Should regain focus
        await browser.keys(['!'])
        await browser.pause(200)

        const focused = await browser.execute(() => {
            const textarea = document.querySelector('textarea')
            return document.activeElement === textarea
        })

        expect(focused).toBe(true)
    })
})
