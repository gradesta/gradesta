/// <reference types="cypress" />

import {get_date_range, get_dates_as_strings} from "../../kcfGraph";

describe("Unit tests", function () {

    before(() => {
        // check if the import worked correctly
        expect(get_date_range, 'get_date_range').to.be.a('function')
    })

    context('kcfGraph.js', function () {
        it("get_date_range doesn't crash when an empty task list is passed.", function () {
            expect(get_date_range([])).to.eq((null, null))
        })

        it("get_date_range works with valid data", function () {
            cy.fixture('tasks').then((tasks) => {
                expect(get_date_range(tasks)).to.eq(("2022-09-01 00:00", "2022-10-23 00:00"))
            })
        })

        it("get_dates_as_strings works with valid data (simultaneously tests get_dates_as_ms)", function () {
            cy.fixture('tasks').then((tasks) => {
                let dates = get_dates_as_strings("2022-09-01 00:00", "2022-09-3 00:00");
                expect(dates[0]).to.eq("2022-09-01")
                expect(dates[1]).to.eq("2022-09-02")
                expect(dates[2]).to.eq("2022-09-03")
            })
        })
    })
})
