/// <reference types="cypress" />

import kcfGraph from "../../kcfGraph";

describe("Unit tests", function () {

    before(() => {
        // check if the import worked correctly
        expect(kcfGraph.get_date_range, 'get_date_range').to.be.a('function')
    })

    context('kcfGraph.js', function () {
        it("get_date_range doesn't crash when an empty task list is passed.", function () {
            expect(kcfGraph.get_date_range([])).to.eq((null, null))
        })

        it("get_date_range works with valid data", function () {
            cy.fixture('tasks').then((tasks) => {
                expect(kcfGraph.get_date_range(tasks)).to.eq(("2022-09-01 00:00", "2022-10-23 00:00"))
            })


        })
    })
})
